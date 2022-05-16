use crate::config;

// run from RAM--this makes it...slower?
// #[inline(never)]
// #[link_section = ".data.ffi::process"]
pub fn process(buf: &[u16; config::ADC_BUF_LEN]) -> (u16, u16) {
    let mut min = u16::MAX;
    let mut max = 0u16;
    for sample in buf.iter().copied() {
        min = min.min(sample);
        max = max.max(sample);
    }
    (min, max)
}
