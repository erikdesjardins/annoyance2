//! Debugging flags

#![allow(clippy::erasing_op)]

use crate::config;

pub const FAKE_INPUT_DATA: bool = false;
pub const FAKE_INPUT_CYCLES_PER_BUF: usize = 8 /* frequency = this * BUFFERS_PER_SEC */;
pub const FAKE_INPUT_PHASE: usize = 0 * u16::MAX as usize / 4 /* phase = 2pi * this / u16::MAX */;
pub const FAKE_INPUT_AMPLITUDE: u16 = config::adc::MAX_POSSIBLE_SAMPLE;

pub const LOG_TIMING: bool = false;

pub const LOG_CONTROL_VALUES: bool = false;

pub const LOG_LAST_FEW_SAMPLES: bool = false;
pub const LOG_LAST_N_SAMPLES: usize = 100;

pub const LOG_FFT_PEAKS: bool = false;

pub const LOG_ALL_FFT_AMPLITUDES: bool = false;

pub const LOG_ALL_PULSES: bool = false;
