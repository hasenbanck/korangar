//! A thread pool that provides an interface like `std::thread::scope`, but
//! re-uses the same threads for the same spawned work.

use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{current, park, Thread};

type StartSignal = Arc<(Mutex<bool>, Condvar)>;
type StopSignal = Arc<AtomicBool>;
type WorkChannel<const MAX_CLOSURE_SIZE: usize> = Arc<Mutex<Work<MAX_CLOSURE_SIZE>>>;

struct Work<const MAX_CLOSURE_SIZE: usize> {
    data: [MaybeUninit<u8>; MAX_CLOSURE_SIZE],
    vtable: *const (),
    call: unsafe fn(*mut ()),
    called: bool,
}

unsafe impl<const MAX_CLOSURE_SIZE: usize> Send for Work<MAX_CLOSURE_SIZE> {}
unsafe impl<const MAX_CLOSURE_SIZE: usize> Sync for Work<MAX_CLOSURE_SIZE> {}

unsafe fn call_closure<F: FnOnce()>(data: *mut ()) {
    let closure = std::ptr::read(data as *mut F);
    closure();
}

unsafe fn noop(_: *mut ()) {}

fn thread_loop<const MAX_CLOSURE_SIZE: usize>(
    data: Arc<ScopeData>,
    start_signal: StartSignal,
    stop_signal: StopSignal,
    work_channel: WorkChannel<MAX_CLOSURE_SIZE>,
) {
    if catch_unwind(AssertUnwindSafe(|| {
        let (lock, condvar) = &*start_signal;

        loop {
            let mut start = lock.lock().unwrap();
            while !*start {
                start = condvar.wait(start).unwrap();
            }

            if stop_signal.load(Ordering::Acquire) {
                break;
            }

            let mut channel = work_channel.lock().unwrap();
            if !channel.called {
                unsafe {
                    (channel.call)(channel.data.as_mut_ptr() as *mut ());
                }
                channel.called = true;
            }

            data.decrement_num_running_threads(false);
            *start = false;
        }
    }))
    .is_err()
    {
        data.decrement_num_running_threads(true);
    }
}

/// A fixed-size thread pool that allows for scoped execution of tasks.
///
/// This thread pool provides an interface similar to `std::thread::scope`,
/// but reuses the same threads for multiple task executions. It allows
/// safe access to stack-local variables from spawned tasks.
///
/// The number of threads in the pool is specified by the `THREAD_COUNT`
/// const generic parameter. Since this implementation will never allocate
/// on the heap, when a closure is spawned, the use need to specify the
/// maximal number of bytes reserved for closure elements by using the
/// const generic parameter `MAX_CLOSURE_SIZE`. The spawning will panic
/// if the closure exceeds this maximum at runtime.
///
/// # Example
///
/// ```
/// use korangar_util::thread::ScopedThreadPool;
///
/// let mut pool = ScopedThreadPool::<4, 8>::new();
/// let mut count: u64 = 0;
///
/// pool.scope(|scope| {
///     scope.spawn::<0, _>(|| {
///         count += 1;
///     });
/// });
///
/// assert_eq!(count, 1);
/// ```
pub struct ScopedThreadPool<const THREAD_COUNT: usize, const MAX_CLOSURE_SIZE: usize> {
    data: Arc<ScopeData>,
    stop_signal: StopSignal,
    start_signals: [StartSignal; THREAD_COUNT],
    work_channels: [WorkChannel<MAX_CLOSURE_SIZE>; THREAD_COUNT],
    // The thread pool are not safe to move between threads.
    _marker: PhantomData<*mut std::ffi::c_void>,
}

impl<const THREAD_COUNT: usize, const MAX_CLOSURE_SIZE: usize> Drop for ScopedThreadPool<THREAD_COUNT, MAX_CLOSURE_SIZE> {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Release);
        self.start_signals.iter().for_each(|start_signal| {
            let (lock, condvar) = &**start_signal;
            {
                if !lock.is_poisoned() {
                    *lock.lock().unwrap() = true;
                    condvar.notify_one();
                }
            }
        })
    }
}

impl<const THREAD_COUNT: usize, const MAX_CLOSURE_SIZE: usize> ScopedThreadPool<THREAD_COUNT, MAX_CLOSURE_SIZE> {
    /// Creates a new scoped thread pool.
    pub fn new() -> Self {
        let data = Arc::new(ScopeData {
            num_running_threads: AtomicUsize::new(0),
            a_thread_panicked: AtomicBool::new(false),
            main_thread: current(),
        });

        let stop_signal = Arc::new(AtomicBool::new(false));
        let start_signals: [StartSignal; THREAD_COUNT] = std::array::from_fn(|_| Arc::new((Mutex::new(false), Condvar::new())));
        let work_channels: [WorkChannel<MAX_CLOSURE_SIZE>; THREAD_COUNT] = std::array::from_fn(|_| {
            Arc::new(Mutex::new(Work {
                data: [MaybeUninit::uninit(); MAX_CLOSURE_SIZE],
                vtable: std::ptr::null_mut(),
                call: noop,
                called: false,
            }))
        });

        (0..THREAD_COUNT).for_each(|thread_num| {
            let data = data.clone();
            let stop_signal = stop_signal.clone();
            let start_signal = start_signals[thread_num].clone();
            let work = work_channels[thread_num].clone();
            std::thread::spawn(move || thread_loop(data, start_signal, stop_signal, work));
        });

        Self {
            data,
            stop_signal,
            work_channels,
            start_signals,
            _marker: PhantomData,
        }
    }

    /// Executes a closure within a scoped thread pool context.
    ///
    /// This method allows for the execution of concurrent tasks that can safely
    /// access stack-local variables from the calling function's scope. It
    /// ensures that all spawned tasks complete before returning,
    /// maintaining Rust's safety guarantees.
    ///
    /// Thread Selection:
    /// When spawning a task, the user explicitly specifies which thread to use
    /// via a const generic parameter in the `spawn` method. For example,
    /// `scope.spawn::<0>(...)` will always use the first thread in the pool,
    /// `scope.spawn::<1>(...)` the second, and so on.
    ///
    /// This explicit thread selection can lead to better CPU cache utilization.
    /// By consistently using the same thread (and likely the same CPU core) for
    /// specific tasks, there's an increased chance of reusing cached data,
    /// potentially improving performance for cache-sensitive workloads.
    pub fn scope<'env, F>(&'env mut self, function: F)
    where
        F: for<'scope> FnOnce(&'scope Scope<'scope, 'env, THREAD_COUNT, MAX_CLOSURE_SIZE>),
    {
        let scope = Scope {
            data: self.data.clone(),
            env: PhantomData,
            scope: PhantomData,
            start_signals: &self.start_signals,
            work_channels: &self.work_channels,
        };

        // Run `function`, but catch panics so we can make sure to wait for all the
        // threads to join.
        let result: std::thread::Result<()> = catch_unwind(AssertUnwindSafe(|| function(&scope)));

        // Wait until all the threads are finished.
        while scope.data.num_running_threads.load(Ordering::Acquire) != 0 {
            park();
        }

        // Throw any panic from `function`, or the return if no thread panicked.
        match result {
            Err(e) => resume_unwind(e),
            Ok(_) if scope.data.a_thread_panicked.load(Ordering::Relaxed) => {
                panic!("a scoped thread panicked")
            }
            Ok(()) => { /* Nothing to do */ }
        }
    }
}

/// Struct used to spawn tasks inside the scope.
pub struct Scope<'scope, 'env: 'scope, const THREAD_COUNT: usize, const MAX_CLOSURE_SIZE: usize> {
    /// Invariance over 'scope, to make sure 'scope cannot shrink,
    /// which is necessary for soundness.
    ///
    /// Without invariance, this would compile fine but be unsound:
    ///
    /// ```compile_fail,E0373
    /// std::thread::scope(|s| {
    ///     s.spawn(|| {
    ///         let a = String::from("abcd");
    ///         s.spawn(|| println!("{a:?}")); // might run after `a` is dropped
    ///     });
    /// });
    /// ```
    scope: PhantomData<&'scope mut &'scope ()>,
    env: PhantomData<&'env mut &'env ()>,
    data: Arc<ScopeData>,
    start_signals: &'env [StartSignal; THREAD_COUNT],
    work_channels: &'env [WorkChannel<MAX_CLOSURE_SIZE>; THREAD_COUNT],
}

impl<'scope, 'env, const THREAD_COUNT: usize, const MAX_CLOSURE_SIZE: usize> Scope<'scope, 'env, THREAD_COUNT, MAX_CLOSURE_SIZE> {
    /// Spawns a new task on the selected thread using `THREAD_NUM`.
    pub fn spawn<const THREAD_NUM: usize, F>(&'scope self, work: F)
    where
        F: FnOnce() + Send + 'scope,
    {
        assert!(size_of::<F>() <= MAX_CLOSURE_SIZE, "Closure too large");

        let start_signal = &self.start_signals[THREAD_NUM];
        let channel = &self.work_channels[THREAD_NUM];
        let mut channel = channel.lock().unwrap();

        unsafe {
            let (_, vtable) = {
                let fat_ptr: &dyn FnOnce() = &work;
                std::mem::transmute::<_, (*mut (), *mut ())>(fat_ptr)
            };

            // Store the closure in the pre-allocated space.
            std::ptr::write(channel.data.as_mut_ptr() as *mut F, work);

            channel.vtable = vtable;
            channel.call = call_closure::<F>;
        }
        channel.called = false;

        let (lock, condvar) = &**start_signal;
        {
            *lock.lock().unwrap() = true;
            self.data.increment_num_running_threads();
            condvar.notify_one();
        }
    }
}

struct ScopeData {
    num_running_threads: AtomicUsize,
    a_thread_panicked: AtomicBool,
    main_thread: Thread,
}

impl ScopeData {
    fn increment_num_running_threads(&self) {
        // We check for 'overflow' with usize::MAX / 2, to make sure there's no
        // chance it overflows to 0, which would result in unsoundness.
        if self.num_running_threads.fetch_add(1, Ordering::Relaxed) > usize::MAX / 2 {
            // This can only reasonably happen by mem::forget()'ing a lot of
            // ScopedJoinHandles.
            self.overflow();
        }
    }

    #[cold]
    fn overflow(&self) {
        self.decrement_num_running_threads(false);
        panic!("too many running threads in thread scope");
    }

    fn decrement_num_running_threads(&self, panic: bool) {
        if panic {
            self.a_thread_panicked.store(true, Ordering::Relaxed);
        }
        // The spawned thread sees a "0" here.
        if self.num_running_threads.fetch_sub(1, Ordering::Release) == 1 {
            self.main_thread.unpark();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_pool_creation() {
        let pool = ScopedThreadPool::<4, 8>::new();
        assert_eq!(pool.start_signals.len(), 4);
        assert_eq!(pool.work_channels.len(), 4);
    }

    #[test]
    fn test_single_spawn_execution() {
        let mut pool = ScopedThreadPool::<4, 8>::new();
        let mut counter = 0;

        pool.scope(|scope| {
            scope.spawn::<0, _>(|| {
                counter += 1;
            });
        });

        assert_eq!(counter, 1);
    }

    #[test]
    #[should_panic]
    fn test_panic_handling() {
        let mut pool = ScopedThreadPool::<1, 8>::new();

        pool.scope(|scope| {
            scope.spawn::<0, _>(|| {
                panic!("Intentional panic");
            });
        });
    }

    #[test]
    fn test_multiple_scope_executions() {
        let mut pool = ScopedThreadPool::<1, 8>::new();

        let mut counter = 0;

        for _ in 0..5 {
            pool.scope(|scope| {
                scope.spawn::<0, _>(|| {
                    counter += 1;
                });
            });
        }

        assert_eq!(counter, 5);
    }

    #[test]
    fn test_all_spawns_executed() {
        let mut pool = ScopedThreadPool::<2, 8>::new();

        let mut touched_0 = false;
        let mut touched_1 = false;

        pool.scope(|scope| {
            scope.spawn::<0, _>(|| {
                touched_0 = true;
            });
            scope.spawn::<1, _>(|| {
                touched_1 = true;
            });
        });

        assert!(touched_0);
        assert!(touched_1);
    }
}
