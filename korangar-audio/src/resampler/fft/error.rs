use std::{error, fmt};

/// The error type returned when constructing [Resampler](crate::Resampler).
pub(crate) enum ResamplerConstructionError {
    InvalidSampleRate { input: usize, output: usize },
    InvalidChunkSize(usize),
}

impl fmt::Display for ResamplerConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidSampleRate { input, output } => write!(
                formatter,
                "Input and output sample rates must both be > 0. Provided input: {input}, provided output: {output}",
            ),
            Self::InvalidChunkSize(provided) => write!(formatter, "Invalid chunk_size provided: {provided}. chunk_size must be >= 1"),
        }
    }
}

impl fmt::Debug for ResamplerConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self}")
    }
}

impl error::Error for ResamplerConstructionError {}

/// The error type used by the resampler.
pub(crate) enum ResampleError {
    /// Error raised when the number of frames in an input buffer is less
    /// than the minimum expected.
    InsufficientInputBufferSize { expected: usize, actual: usize },
    /// Error raised when the number of frames in an output buffer is less
    /// than the minimum expected.
    InsufficientOutputBufferSize { expected: usize, actual: usize },
}

impl fmt::Display for ResampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientInputBufferSize { expected, actual } => {
                write!(f, "Insufficient input buffer size {actual}, expected {expected} frames")
            }
            Self::InsufficientOutputBufferSize { expected, actual } => {
                write!(f, "Insufficient output buffer size {actual}, expected {expected} frames")
            }
        }
    }
}

impl fmt::Debug for ResampleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self}")
    }
}

impl error::Error for ResampleError {}
