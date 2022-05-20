use crate::config;
use crate::fixed::{amplitude, phase};
use core::mem;
use num_complex::Complex;

mod fft;
mod window;

#[inline(never)]
pub fn process_buffer(
    input: &[u16; config::ADC_BUF_LEN],
    scratch: &mut [i16; config::FFT_BUF_LEN],
) {
    // convert unsigned samples (0 = 0V, u16::MAX = Vcc) to signed samples centered at Vcc/2 (i16::MIN = 0V, 0 = Vcc/2, i16::MAX = Vcc)
    for i in 0..config::ADC_BUF_LEN {
        scratch[i] = (input[i] as i16).wrapping_sub(i16::MAX / 2);
    }

    // zero remaining buffer (to get up to power-of-2)
    // apparently you can pad your sample with zeroes and this increases frequency resolution?
    // spectral interpolation is magic
    for i in config::ADC_BUF_LEN..config::FFT_BUF_LEN {
        scratch[i] = 0;
    }

    // apply window function
    let nonzero_samples: &mut [i16; config::ADC_BUF_LEN] =
        (&mut scratch[0..config::ADC_BUF_LEN]).try_into().unwrap();
    match config::FFT_WINDOW {
        config::Window::Rectangle => window::rectangle(nonzero_samples),
        config::Window::BlackmanHarris => window::blackman_harris(nonzero_samples),
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
