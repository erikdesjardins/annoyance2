use crate::config;

#[inline(never)]
pub fn differential_to_single_ended(
    input: &[u16; config::adc::BUF_LEN_PER_CHANNEL * 2],
    output: &mut [i16; config::adc::BUF_LEN_PER_CHANNEL],
) {
    // convert unsigned differential samples (centered individually at Vcc/2) to signed samples (centered at 0)
    for (value, channels) in output.iter_mut().zip(input.chunks_exact(2)) {
        // subtracting the two channels cancels out the common Vcc/2 offset
        let difference = i32::from(channels[1]) - i32::from(channels[0]);
        // saturate for differences that can't fit into i16 (can overflow by up to 1 bit)
        // as an alternative to this, we could shift out one bit, but that would lose resolution
        *value = difference
            .try_into()
            .unwrap_or(if difference < 0 { i16::MIN } else { i16::MAX });
    }
}
