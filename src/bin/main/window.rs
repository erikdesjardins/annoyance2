use crate::config;
use crate::fixed::scale_by;

// put in RAM: ~100us improvement
// #[link_section = ".data.adc::window::HAMMING"]
static HAMMING: [u16; config::adc::BUF_LEN_PER_CHANNEL] =
    include!(concat!(env!("OUT_DIR"), "/hamming.rs"));

static BLACKMAN_NUTALL: [u16; config::adc::BUF_LEN_PER_CHANNEL] =
    include!(concat!(env!("OUT_DIR"), "/blackman_nutall.rs"));

static BLACKMAN_HARRIS: [u16; config::adc::BUF_LEN_PER_CHANNEL] =
    include!(concat!(env!("OUT_DIR"), "/blackman_harris.rs"));

#[inline(never)]
pub fn apply_to(data: &mut [i16; config::adc::BUF_LEN_PER_CHANNEL]) {
    let window = match config::fft::WINDOW {
        config::fft::Window::Rectangle => {
            // no scaling
            return;
        }
        config::fft::Window::Hamming => HAMMING,
        config::fft::Window::BlackmanNutall => BLACKMAN_NUTALL,
        config::fft::Window::BlackmanHarris => BLACKMAN_HARRIS,
    };

    assert_eq!(data.len(), window.len());

    for (x, scale) in data.iter_mut().zip(window) {
        *x = scale_by(*x, scale);
    }
}
