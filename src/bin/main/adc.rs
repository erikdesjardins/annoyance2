use crate::config;
use crate::fixed::{amplitude_squared, phase, scale_by, sqrt};
use crate::panic::OptionalExt;
use core::mem;
use num_complex::Complex;

mod fft;
mod window;

#[inline(never)]
pub fn process_buffer(
    input: &[u16; config::adc::BUF_LEN_PER_CHANNEL * 2],
    scratch: &mut [i16; config::fft::BUF_LEN_REAL],
) {
    debug_log_final_samples(input);

    let (values, padding) = scratch.split_at_mut(config::adc::BUF_LEN_PER_CHANNEL);
    let values: &mut [i16; config::adc::BUF_LEN_PER_CHANNEL] =
        values.try_into().unwrap_infallible();

    // convert unsigned differential samples (centered individually at Vcc/2) to signed samples (centered at 0)
    for (value, channels) in values.iter_mut().zip(input.chunks_exact(2)) {
        // subtracting the two channels cancels out the common Vcc/2 offset
        let difference = i32::from(channels[1]) - i32::from(channels[0]);
        // saturate for differences that can't fit into i16 (can overflow by up to 1 bit)
        // as an alternative to this, we could shift out one bit, but that would lose resolution
        *value = difference
            .try_into()
            .unwrap_or(if difference < 0 { i16::MIN } else { i16::MAX });
    }

    // zero remaining buffer (to get up to power-of-2)
    // apparently you can pad your sample with zeroes and this increases frequency resolution?
    // spectral interpolation is magic
    padding.fill(0);

    // apply window function
    let window_fn = match config::fft::WINDOW {
        config::fft::Window::Rectangle => window::rectangle,
        config::fft::Window::BlackmanHarris => window::blackman_harris,
    };
    window_fn(values);

    // run fft
    let data = complex_from_adjacent_values(scratch);
    fft::radix2(data);

    debug_log_fft_stats(data);
}

fn complex_from_adjacent_values<T>(
    x: &mut [T; config::fft::BUF_LEN_REAL],
) -> &mut [Complex<T>; config::fft::BUF_LEN_COMPLEX] {
    const _: () = assert!(config::fft::BUF_LEN_REAL == 2 * config::fft::BUF_LEN_COMPLEX);
    // Safety: Complex<T> is layout-compatible with [T; 2]
    unsafe { mem::transmute(x) }
}

#[inline(never)]
fn debug_log_final_samples(input: &[u16; config::adc::BUF_LEN_PER_CHANNEL * 2]) {
    if !config::debug::LOG_FINAL_ADC_SAMPLES {
        return;
    }

    defmt::info!("Final samples: {}", &input[input.len() - 4..]);
}

#[inline(never)]
fn debug_log_fft_stats(data: &mut [Complex<i16>; config::fft::BUF_LEN_COMPLEX]) {
    if !config::debug::LOG_FFT_STATS {
        return;
    }

    let mut max_amplitude_squared = 0;
    let mut i_at_max = 0;
    let mut val_at_max = Complex::new(0, 0);

    // only look at positive, non-DC frequencies in first half of array
    for i in 2..data.len() / 2 {
        let amplitude_squared = amplitude_squared(data[i]);
        if amplitude_squared > max_amplitude_squared {
            max_amplitude_squared = amplitude_squared;
            i_at_max = i;
            val_at_max = data[i];
        }
    }

    let max_amplitude = sqrt(max_amplitude_squared);
    let freq_at_max = i_at_max * config::fft::FREQ_RESOLUTION_X1000 / 1000;
    let phase_at_max = phase(val_at_max);
    let deg_at_max = scale_by(360, (phase_at_max >> 16) as u16);

    defmt::info!(
        "Max amplitude = {} @ freq = {} Hz, phase = {}/{} cycles (~{} deg)",
        max_amplitude,
        freq_at_max,
        phase_at_max,
        u32::MAX,
        deg_at_max,
    );
}
