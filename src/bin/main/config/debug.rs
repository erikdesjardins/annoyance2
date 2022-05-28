//! Debugging flags

pub const FAKE_INPUT_DATA: bool = false;
pub const FAKE_INPUT_CYCLES_PER_BUF: usize = 8 /* frequency = this * BUFFERS_PER_SEC */;
pub const FAKE_INPUT_AMPLITUDE: u16 = u16::MAX / 2;

pub const LOG_LAST_FEW_SAMPLES: bool = false;
pub const LOG_LAST_N_SAMPLES: usize = 100;

pub const LOG_ALL_FFT_AMPLITUDES: bool = false;
