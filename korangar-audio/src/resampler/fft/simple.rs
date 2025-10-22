use std::f32::consts::PI;

/// A complex number represented as interleaved real and imaginary parts.
/// This layout is SIMD-friendly because real and imaginary parts are adjacent
/// in memory.
#[derive(Clone, Copy, Debug)]
pub struct Complex32 {
    pub re: f32,
    pub im: f32,
}

impl Complex32 {
    #[inline]
    pub fn new(re: f32, im: f32) -> Self {
        Self { re, im }
    }

    #[inline]
    pub fn zero() -> Self {
        Self { re: 0.0, im: 0.0 }
    }

    /// Complex multiplication: (a + bi) * (c + di) = (ac - bd) + (ad + bc)i
    /// This will be one of the main targets for SIMD optimization later.
    #[inline]
    pub fn mul(&self, other: &Complex32) -> Complex32 {
        Complex32 {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }

    #[inline]
    pub fn add(&self, other: &Complex32) -> Complex32 {
        Complex32 {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }

    #[inline]
    pub fn sub(&self, other: &Complex32) -> Complex32 {
        Complex32 {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }

    #[inline]
    pub fn conj(&self) -> Complex32 {
        Complex32 { re: self.re, im: -self.im }
    }

    #[inline]
    pub fn scale(&self, scalar: f32) -> Complex32 {
        Complex32 {
            re: self.re * scalar,
            im: self.im * scalar,
        }
    }
}

/// Real FFT planner and executor for power-of-two lengths.
/// This handles both forward (real -> complex) and inverse (complex -> real)
/// transforms.
pub struct RealFFT {
    /// Length of the real input/output (must be power of 2)
    length: usize,

    /// Pre-computed twiddle factors for the complex FFT stage.
    /// These are the roots of unity: e^(-2πik/N) for k = 0..N/2
    twiddle_factors: Vec<Complex32>,

    /// Pre-computed twiddle factors for the real-to-complex packing/unpacking.
    /// These handle the conversion between N real samples and N/2+1 complex
    /// coefficients.
    real_twiddles: Vec<Complex32>,

    /// Bit-reversed indices for in-place FFT computation.
    /// This allows us to avoid recursive calls by pre-computing the correct
    /// order.
    bit_reversed_indices: Vec<usize>,
}

impl RealFFT {
    /// Create a new RealFFT for a given length (must be power of 2).
    pub fn new(length: usize) -> Self {
        assert!(length.is_power_of_two(), "Length must be a power of 2");
        assert!(length >= 2, "Length must be at least 2");

        let half_length = length / 2;

        // Precompute twiddle factors for the complex FFT (operates on length/2 complex
        // numbers)
        let twiddle_factors = Self::compute_twiddle_factors(half_length);

        // Precompute twiddle factors for real-to-complex conversion
        let real_twiddles = Self::compute_real_twiddles(length);

        // Precompute bit-reversed indices
        let bit_reversed_indices = Self::compute_bit_reversal(half_length);

        Self {
            length,
            twiddle_factors,
            real_twiddles,
            bit_reversed_indices,
        }
    }

    /// Compute twiddle factors: W_N^k = e^(-2πik/N)
    /// These are the complex roots of unity used in the FFT butterfly
    /// operations.
    fn compute_twiddle_factors(n: usize) -> Vec<Complex32> {
        (0..n)
            .map(|k| {
                let angle = -2.0 * PI * (k as f32) / (n as f32);
                Complex32::new(angle.cos(), angle.sin())
            })
            .collect()
    }

    /// Compute special twiddle factors for real-to-complex packing.
    /// These handle the Hermitian symmetry exploitation.
    fn compute_real_twiddles(n: usize) -> Vec<Complex32> {
        (0..=n / 2)
            .map(|k| {
                let angle = -2.0 * PI * (k as f32) / (n as f32);
                Complex32::new(angle.cos(), angle.sin())
            })
            .collect()
    }

    /// Compute bit-reversed indices for in-place FFT.
    /// This reordering allows us to use an iterative algorithm instead of
    /// recursion.
    fn compute_bit_reversal(n: usize) -> Vec<usize> {
        let num_bits = n.trailing_zeros() as usize;
        (0..n).map(|i| Self::reverse_bits(i, num_bits)).collect()
    }

    /// Reverse the bits of an integer (used for bit-reversal permutation).
    fn reverse_bits(mut x: usize, num_bits: usize) -> usize {
        let mut result = 0;
        for _ in 0..num_bits {
            result = (result << 1) | (x & 1);
            x >>= 1;
        }
        result
    }

    /// Forward FFT: Transform real input to complex frequency domain.
    /// Input: slice of real f32 values (length N)
    /// Output: slice of Complex32 values (length N/2 + 1) representing
    /// non-redundant spectrum
    pub fn process_forward(&self, input: &[f32], output: &mut [Complex32]) {
        assert_eq!(input.len(), self.length, "Input length mismatch");
        assert_eq!(output.len(), self.length / 2 + 1, "Output length mismatch");

        let half_length = self.length / 2;

        // Step 1: Pack real input into complex array (treating pairs as complex
        // numbers) We interpret the N real samples as N/2 complex samples for
        // the first FFT stage.
        let mut packed = vec![Complex32::zero(); half_length];
        for i in 0..half_length {
            packed[i] = Complex32::new(input[2 * i], input[2 * i + 1]);
        }

        // Step 2: Perform complex FFT on the packed data
        self.complex_fft_inplace(&mut packed);

        // Step 3: Unpack the result to exploit Hermitian symmetry
        // This converts the N/2 complex FFT output into the N/2+1 real FFT output.
        self.unpack_forward(&packed, output);
    }

    /// Inverse FFT: Transform complex frequency domain back to real time
    /// domain. Input: slice of Complex32 values (length N/2 + 1)
    /// Output: slice of real f32 values (length N)
    pub fn process_inverse(&self, input: &[Complex32], output: &mut [f32]) {
        assert_eq!(input.len(), self.length / 2 + 1, "Input length mismatch");
        assert_eq!(output.len(), self.length, "Output length mismatch");

        let half_length = self.length / 2;

        // Step 1: Pack the Hermitian-symmetric spectrum into complex array
        let mut packed = vec![Complex32::zero(); half_length];
        self.pack_inverse(input, &mut packed);

        // Step 2: Perform inverse complex FFT
        self.complex_ifft_inplace(&mut packed);

        // Step 3: Unpack the complex result into real output
        for i in 0..half_length {
            output[2 * i] = packed[i].re;
            output[2 * i + 1] = packed[i].im;
        }
    }

    /// Core complex FFT using Cooley-Tukey radix-2 decimation-in-time
    /// algorithm. This is performed in-place for memory efficiency.
    fn complex_fft_inplace(&self, data: &mut [Complex32]) {
        let n = data.len();

        // Step 1: Bit-reversal permutation
        // This reorders the input so we can process iteratively instead of recursively.
        for i in 0..n {
            let j = self.bit_reversed_indices[i];
            if i < j {
                data.swap(i, j);
            }
        }

        // Step 2: Iterative FFT using butterfly operations
        // We process in stages, where each stage doubles the size of the DFT blocks.
        let mut block_size = 2;
        while block_size <= n {
            let half_block = block_size / 2;

            // Twiddle factor step size for this stage
            let twiddle_step = n / block_size;

            // Process each block at this stage
            for block_start in (0..n).step_by(block_size) {
                // Apply butterfly operations within this block
                for i in 0..half_block {
                    let k = i * twiddle_step;
                    let twiddle = self.twiddle_factors[k];

                    let idx_top = block_start + i;
                    let idx_bottom = idx_top + half_block;

                    // Butterfly operation: This is the core of the FFT
                    // top_new = top + twiddle * bottom
                    // bottom_new = top - twiddle * bottom
                    let temp = twiddle.mul(&data[idx_bottom]);
                    let top = data[idx_top];

                    data[idx_top] = top.add(&temp);
                    data[idx_bottom] = top.sub(&temp);
                }
            }

            block_size *= 2;
        }
    }

    /// Inverse complex FFT (same as forward but with conjugated twiddles and
    /// scaling).
    fn complex_ifft_inplace(&self, data: &mut [Complex32]) {
        let n = data.len();

        // Conjugate input
        for x in data.iter_mut() {
            *x = x.conj();
        }

        // Perform forward FFT
        self.complex_fft_inplace(data);

        // Conjugate and scale output
        let scale = 1.0 / (n as f32);
        for x in data.iter_mut() {
            *x = x.conj().scale(scale);
        }
    }

    /// Unpack complex FFT output to get real FFT output (forward transform).
    /// This exploits the Hermitian symmetry of real-valued signals.
    fn unpack_forward(&self, packed: &[Complex32], output: &mut [Complex32]) {
        let n = self.length;
        let half_n = n / 2;

        // DC component (k=0) is special: purely real
        output[0] = Complex32::new(packed[0].re + packed[0].im, 0.0);

        // Nyquist component (k=N/2) is also special: purely real
        output[half_n] = Complex32::new(packed[0].re - packed[0].im, 0.0);

        // Process intermediate frequencies using symmetry
        for k in 1..half_n {
            let fk = packed[k];
            let fnk = packed[half_n - k].conj();

            let w = self.real_twiddles[k];

            // These formulas come from the identity that relates the FFT of packed
            // complex data to the FFT of the original real data
            let sum = fk.add(&fnk);
            let diff = fk.sub(&fnk);

            let temp = Complex32::new(diff.im, -diff.re).mul(&w);

            output[k] = sum.add(&temp).scale(0.5);
        }
    }

    /// Pack real FFT input for inverse transform (inverse transform).
    /// This is the reverse of unpack_forward.
    fn pack_inverse(&self, input: &[Complex32], packed: &mut [Complex32]) {
        let n = self.length;
        let half_n = n / 2;

        // DC and Nyquist components
        let f0 = input[0].re;
        let f_n = input[half_n].re;
        packed[0] = Complex32::new((f0 + f_n) * 0.5, (f0 - f_n) * 0.5);

        // Intermediate frequencies
        for k in 1..half_n {
            let fk = input[k];
            let w = self.real_twiddles[k].conj();

            // We need to compute the conjugate symmetry
            let fnk = if k < input.len() {
                input[half_n - k].conj()
            } else {
                Complex32::zero()
            };

            let sum = fk.add(&fnk);
            let diff = fk.sub(&fnk);

            let temp = Complex32::new(diff.im, -diff.re).mul(&w);

            packed[k] = sum.sub(&temp);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_inverse_roundtrip() {
        let length = 16;
        let fft = RealFFT::new(length);

        // Create test signal: sum of two sine waves
        let input: Vec<f32> = (0..length)
            .map(|i| {
                let t = i as f32 / length as f32;
                (2.0 * PI * 2.0 * t).sin() + 0.5 * (2.0 * PI * 3.0 * t).sin()
            })
            .collect();

        // Forward transform
        let mut spectrum = vec![Complex32::zero(); length / 2 + 1];
        fft.process_forward(&input, &mut spectrum);

        // Inverse transform
        let mut output = vec![0.0; length];
        fft.process_inverse(&spectrum, &mut output);

        // Check roundtrip accuracy
        for (i, (&expected, &actual)) in input.iter().zip(output.iter()).enumerate() {
            assert!(
                (expected - actual).abs() < 1e-5,
                "Mismatch at index {}: expected {}, got {}",
                i,
                expected,
                actual
            );
        }
    }

    #[test]
    fn test_dc_signal() {
        let length = 8;
        let fft = RealFFT::new(length);

        // DC signal (constant value)
        let input = vec![1.0; length];
        let mut spectrum = vec![Complex32::zero(); length / 2 + 1];
        fft.process_forward(&input, &mut spectrum);

        // All energy should be at DC
        assert!((spectrum[0].re - length as f32).abs() < 1e-5);
        assert!(spectrum[0].im.abs() < 1e-5);

        for i in 1..spectrum.len() {
            assert!(spectrum[i].re.abs() < 1e-5);
            assert!(spectrum[i].im.abs() < 1e-5);
        }
    }
}
