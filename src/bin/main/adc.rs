use crate::config;
use core::mem;
use num_complex::Complex;

mod fft;
mod window;

#[inline(never)]
pub fn process_buffer(
    input: &[u16; config::ADC_BUF_LEN],
    scratch: &mut [i16; config::FFT_BUF_LEN],
) -> (i16, i16) {
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

    let mut max_i = 0;
    let mut max = i16::MIN;

    for (i, freq) in data.iter().copied().enumerate() {
        if freq.re > max {
            max_i = i as i16;
            max = freq.re;
        }
    }

    (max_i, max)
}

fn complex_from_adjacent_values<T>(
    x: &mut [T; config::FFT_BUF_LEN],
) -> &mut [Complex<T>; config::FFT_BUF_LEN / 2] {
    assert!(x.len() % 2 == 0);
    // Safety: Complex<T> is layout-compatible with [T; 2]
    unsafe { mem::transmute(x) }
}
