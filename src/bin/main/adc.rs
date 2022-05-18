use crate::config;
use core::mem;
use num_complex::Complex;

mod fft;

#[inline(never)]
pub fn process(
    buf: &[u16; config::ADC_BUF_LEN],
    scratch: &mut [i16; config::FFT_BUF_LEN],
) -> (i16, i16) {
    // rescale so the midpoint is zero (since the signal is differential)
    for i in 0..config::ADC_BUF_LEN {
        scratch[i] = (buf[i] as i16).wrapping_sub(i16::MAX / 2);
    }
    // zero remaining buffer--it needs to have power-of-2 len
    // also, apparently you can pad your sample with zeroes and this increases frequency resolution?
    // spectral interpolation is magic
    for i in config::ADC_BUF_LEN..config::FFT_BUF_LEN {
        scratch[i] = 0;
    }

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
