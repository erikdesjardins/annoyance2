use crate::config;
use crate::math::ScaleBy;

// put in RAM: ~100us improvement
// #[link_section = ".data.adc::window::HAMMING"]
const HAMMING: &[u16; config::adc::BUF_LEN_PROCESSED] =
    &include!(concat!(env!("OUT_DIR"), "/hamming.rs"));

const HANN: &[u16; config::adc::BUF_LEN_PROCESSED] =
    &include!(concat!(env!("OUT_DIR"), "/hann.rs"));

const BLACKMAN: &[u16; config::adc::BUF_LEN_PROCESSED] =
    &include!(concat!(env!("OUT_DIR"), "/blackman.rs"));

#[inline(never)]
pub fn apply_to(data: &mut [i16; config::adc::BUF_LEN_PROCESSED]) {
    let window = match config::fft::WINDOW {
        config::fft::Window::Rectangle => {
            // no scaling
            return;
        }
        config::fft::Window::Hamming => HAMMING,
        config::fft::Window::Hann => HANN,
        config::fft::Window::Blackman => BLACKMAN,
    };

    assert_eq!(data.len(), window.len());

    for (x, &scale) in data.iter_mut().zip(window) {
        *x = x.scale_by(scale);
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

    let avg = sum / window.len() as u32;

    avg as u16
}
