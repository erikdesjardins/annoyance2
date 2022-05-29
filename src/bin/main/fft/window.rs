use crate::config;
use crate::fixed::scale_by;

// put in RAM: ~100us improvement
// #[link_section = ".data.adc::window::HAMMING"]
static HAMMING: [u16; config::adc::BUF_LEN_PROCESSED] =
    include!(concat!(env!("OUT_DIR"), "/hamming.rs"));

static HANN: [u16; config::adc::BUF_LEN_PROCESSED] = include!(concat!(env!("OUT_DIR"), "/hann.rs"));

static BLACKMAN: [u16; config::adc::BUF_LEN_PROCESSED] =
    include!(concat!(env!("OUT_DIR"), "/blackman.rs"));

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

    for (x, scale) in data.iter_mut().zip(window) {
        *x = scale_by(*x, scale);
    }
}
