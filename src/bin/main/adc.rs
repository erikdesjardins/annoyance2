use crate::config;
use crate::fixed::{amplitude, phase};
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
    values
        .iter_mut()
        .zip(input.chunks_exact(2))
        .for_each(|(value, channels)| {
            // subtracting the two signals naturally cancels out the Vcc/2 offset
            let difference = channels[1] as i32 - channels[0] as i32;
            debug_assert!(difference >= i16::MIN as i32);
            debug_assert!(difference <= i16::MAX as i32);
            *value = difference as i16;
        });

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

    let mut max_amplitude = 0;
    let mut freq_at_max = 0;
    let mut phase_at_max = 0;
    // only look at positive, non-DC frequencies in first half of array
    for i in 2..data.len() / 2 {
        let freq = i as u32 * config::FFT_FREQ_RESOLUTION_X1000 / 1000;
        let amplitude = amplitude(data[i]);
        let phase = phase(data[i]);

        if amplitude > max_amplitude {
            max_amplitude = amplitude;
            freq_at_max = freq;
            phase_at_max = phase;
        }
    }
    defmt::info!(
        "Max amplitude = {} @ freq = {} Hz, phase = {}/{} cycles",
        max_amplitude,
        freq_at_max,
        phase_at_max,
        u16::MAX,
    );
}

fn complex_from_adjacent_values<T>(
    x: &mut [T; config::FFT_BUF_LEN],
) -> &mut [Complex<T>; config::FFT_BUF_LEN / 2] {
    assert!(x.len() % 2 == 0);
    // Safety: Complex<T> is layout-compatible with [T; 2]
    unsafe { mem::transmute(x) }
}
