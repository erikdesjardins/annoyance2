use crate::config;
use crate::fixed::scale_by;
use crate::panic::OptionalExt;

#[inline(never)]
pub fn process_raw_samples(
    input: &[u16; config::adc::BUF_LEN_RAW],
    output: &mut [i16; config::adc::BUF_LEN_PROCESSED],
) {
    // convert unsigned differential samples (centered individually at Vcc/2) to signed samples (centered at 0)
    assert_eq!(output.len() * 2 * config::adc::OVERSAMPLE, input.len());

    for (value, samples) in output
        .iter_mut()
        .zip(input.chunks_exact(2 * config::adc::OVERSAMPLE))
    {
        // sum up oversampled channels
        let mut channel_a: i32 = 0;
        let mut channel_b: i32 = 0;
        for channels in samples.chunks_exact(2) {
            channel_a += i32::from(channels[0]);
            channel_b += i32::from(channels[1]);
        }
        // subtract groups of samples
        let difference = channel_b - channel_a;
        // scale down difference by oversample ratio
        let oversample: i32 = config::adc::OVERSAMPLE.try_into().unwrap_infallible();
        let difference: i32 = difference / oversample;
        // truncate difference, which should fit because ADC only has 12 bits of resolution (hence max difference is 2^12)
        let difference: i16 = difference.try_into().unwrap_or_else(|_| {
            if cfg!(debug_assertions) {
                panic!("overflow in difference truncation: {}", difference);
            } else {
                0
            }
        });
        *value = difference;
    }

    if config::debug::FAKE_INPUT_DATA {
        output.copy_from_slice(&FAKE_COS_TABLE);
    }

    if config::debug::LOG_LAST_FEW_SAMPLES {
        defmt::println!(
            "ADC samples (last {}): {}",
            config::debug::LOG_LAST_N_SAMPLES,
            output[output.len() - config::debug::LOG_LAST_N_SAMPLES..]
        );
    }
}

static FAKE_COS_TABLE: [i16; config::adc::BUF_LEN_PROCESSED] = {
    const LEN: usize = config::adc::BUF_LEN_PROCESSED;

    const COS_TABLE: [i16; LEN] = include!(concat!(env!("OUT_DIR"), "/fake_cos_table.rs"));

    let mut fake = [0; LEN];

    let mut i = 0;
    while i < LEN {
        let frequency = i * config::debug::FAKE_INPUT_CYCLES_PER_BUF;
        let phase = config::debug::FAKE_INPUT_PHASE * LEN / u16::MAX as usize;
        let unscaled_sample = COS_TABLE[(frequency + phase) % LEN];
        fake[i] = scale_by(unscaled_sample, config::debug::FAKE_INPUT_AMPLITUDE);
        i += 1;
    }

    fake
};
