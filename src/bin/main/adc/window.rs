use crate::config;
use crate::fixed::scale_by;

#[inline(never)]
pub fn rectangle(_data: &mut [i16; config::ADC_BUF_LEN]) {
    // rectangle window does nothing
}

// put in RAM: ~100us improvement
#[link_section = ".data.adc::window::BLACKMAN_HARRIS"]
static BLACKMAN_HARRIS: [u16; config::ADC_BUF_LEN] =
    include!(concat!(env!("OUT_DIR"), "/blackman_harris.rs"));

#[inline(never)]
pub fn blackman_harris(data: &mut [i16; config::ADC_BUF_LEN]) {
    for (x, &scale) in data.iter_mut().zip(&BLACKMAN_HARRIS) {
        *x = scale_by(*x, scale);
    }
}
