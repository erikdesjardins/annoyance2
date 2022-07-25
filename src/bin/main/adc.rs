use crate::config;
use crate::math::{const_scale_by_i16_u16, DivRound, ScalingFactor, Truncate};
use crate::panic::OptionalExt;

#[inline(never)]
pub fn process_raw_samples(
    input: &[u16; config::adc::BUF_LEN_RAW],
    output: &mut [i16; config::adc::BUF_LEN_PROCESSED],
) {
    // convert unsigned samples (centered at Vcc/2) to signed samples (centered at 0)
    assert_eq!(output.len() * config::adc::OVERSAMPLE, input.len());

    for (value, samples) in output
        .iter_mut()
        .zip(input.chunks_exact(config::adc::OVERSAMPLE))
    {
        // sum up oversampled samples
        let sample: i32 = samples.iter().copied().map(i32::from).sum();
        // scale down sum by oversample ratio, rounded
        let oversample: i32 = config::adc::OVERSAMPLE.try_into().unwrap_infallible();
        let sample: i32 = sample.div_round(oversample);
        // subtract Vcc/2 offset
        let max_possible_sample: i32 = config::adc::MAX_POSSIBLE_SAMPLE
            .try_into()
            .unwrap_infallible();
        let offset: i32 = max_possible_sample / 2;
        let sample: i32 = sample - offset;
        // truncate sum, which should fit into i16 because ADC has < 16 bits of (unsigned) resolution
        assert!(config::adc::RESOLUTION_BITS < i16::BITS);
        let sample: i16 = sample.truncate();
        *value = sample;
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
        // since we know the samples are already in a 0..u16::MAX range,
        // we can use the desired amplitude directly as a scaling factor
        let factor = ScalingFactor::from_raw(config::debug::FAKE_INPUT_AMPLITUDE);
        fake[i] = const_scale_by_i16_u16(unscaled_sample, factor);
        i += 1;
    }

    fake
};
