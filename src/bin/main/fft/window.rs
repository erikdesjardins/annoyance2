use crate::config;
use crate::math::ScaleBy;

// put in RAM: ~100us improvement
// #[link_section = ".data.adc::window::RECTANGLE"]
const RECTANGLE: &[u16; config::adc::BUF_LEN_PROCESSED] =
    &[u16::MAX; config::adc::BUF_LEN_PROCESSED];

const HAMMING: &[u16; config::adc::BUF_LEN_PROCESSED] =
    &include!(concat!(env!("OUT_DIR"), "/hamming.rs"));

const HANN: &[u16; config::adc::BUF_LEN_PROCESSED] =
    &include!(concat!(env!("OUT_DIR"), "/hann.rs"));

const BLACKMAN: &[u16; config::adc::BUF_LEN_PROCESSED] =
    &include!(concat!(env!("OUT_DIR"), "/blackman.rs"));

#[inline(never)]
pub fn apply_with_scaling(data: &mut [i16; config::adc::BUF_LEN_PROCESSED]) {
    let window = match config::fft::WINDOW {
        config::fft::Window::Rectangle => RECTANGLE,
        config::fft::Window::Hamming => HAMMING,
        config::fft::Window::Hann => HANN,
        config::fft::Window::Blackman => BLACKMAN,
    };

    assert_eq!(data.len(), window.len());

    for (x, &scale) in data.iter_mut().zip(window) {
        // scale up samples to use full i16 range,
        // to keep as much precision as possible when applying the window function
        // and running the FFT (which scales down the samples each stage)
        let full_range = *x << (i16::BITS - u32::from(config::adc::RESOLUTION_BITS));
        let windowed = full_range.scale_by(scale);
        *x = windowed;
    }
}

pub const fn amplitude_scale_factor() -> u16 {
    let window = match config::fft::WINDOW {
        config::fft::Window::Rectangle => {
            // no scaling
            return u16::MAX;
        }
        config::fft::Window::Hamming => HAMMING,
        config::fft::Window::Hann => HANN,
        config::fft::Window::Blackman => BLACKMAN,
    };

    let mut sum: u32 = 0;

    let mut i = 0;
    while i < window.len() {
        sum += window[i] as u32;
        i += 1;
    }

    #[allow(clippy::cast_possible_truncation)]
    let avg = sum / window.len() as u32;

    #[allow(clippy::cast_possible_truncation)]
    let avg = avg as u16;

    avg
}
