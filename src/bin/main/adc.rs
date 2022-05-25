use crate::config;
use crate::fixed::{amplitude_squared, phase, scale_by, sqrt};
use core::mem;
use num_complex::Complex;

mod fft;
mod window;

#[inline(never)]
pub fn process_buffer(
    input: &[u16; config::ADC_BUF_LEN_PER_CHANNEL * 2],
    scratch: &mut [i16; config::FFT_BUF_LEN],
) {
    let (values, padding) = scratch.split_at_mut(config::ADC_BUF_LEN_PER_CHANNEL);
    let values: &mut [i16; config::ADC_BUF_LEN_PER_CHANNEL] = values.try_into().unwrap();

    // convert unsigned differential samples (centered individually at Vcc/2) to signed samples (centered at 0)
    for (value, channels) in values.iter_mut().zip(input.chunks_exact(2)) {
        // subtracting the two channels cancels out the common Vcc/2 offset
        let difference = i32::from(channels[1]) - i32::from(channels[0]);
        debug_assert!(difference >= i32::from(i16::MIN));
        debug_assert!(difference <= i32::from(i16::MAX));
        *value = difference as i16;
    }

    // zero remaining buffer (to get up to power-of-2)
    // apparently you can pad your sample with zeroes and this increases frequency resolution?
    // spectral interpolation is magic
    padding.fill(0);

    // apply window function
    match config::FFT_WINDOW {
        config::Window::Rectangle => window::rectangle(values),
        config::Window::BlackmanHarris => window::blackman_harris(values),
    }

    // run fft
    let data = complex_from_adjacent_values(scratch);
    fft::radix2(data);

    log_fft_stats(data);
}

fn complex_from_adjacent_values<T>(
    x: &mut [T; config::FFT_BUF_LEN],
) -> &mut [Complex<T>; config::FFT_BUF_LEN / 2] {
    assert!(x.len() % 2 == 0);
    // Safety: Complex<T> is layout-compatible with [T; 2]
    unsafe { mem::transmute(x) }
}

#[inline(never)]
fn log_fft_stats(data: &mut [Complex<i16>; config::FFT_BUF_LEN / 2]) {
    let mut max_amplitude_squared = 0;
    let mut i_at_max = 0;
    let mut val_at_max = Complex::new(0, 0);

    // only look at positive, non-DC frequencies in first half of array
    for i in 2..data.len() / 2 {
        let amplitude_squared = amplitude_squared(data[i]);
        if amplitude_squared > max_amplitude_squared {
            max_amplitude_squared = amplitude_squared;
            i_at_max = i as u32;
            val_at_max = data[i];
        }
    }

    let max_amplitude = sqrt(max_amplitude_squared);
    let freq_at_max = i_at_max * config::FFT_FREQ_RESOLUTION_X1000 / 1000;
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
