pub fn dump_to_log() {
    defmt::info!(
        "\n\
        Debugging flags:\n\
        - FAKE_INPUT_DATA:           {}\n\
        - FAKE_INPUT_CYCLES_PER_BUF: {} ({} Hz)\n\
        - FAKE_INPUT_AMPLITUDE:      {}\n\
        - LOG_TIMING: {}\n\
        - LOG_CONTROL_VALUES: {}\n\
        - LOG_LAST_FEW_SAMPLES: {}\n\
        - LOG_LAST_N_SAMPLES:   {}\n\
        - LOG_FFT_PEAKS: {}\n\
        - LOG_ALL_FFT_AMPLITUDES: {}\n\
        - LOG_ALL_PULSES: {}\n\
        Clocks:\n\
        - HSE_FREQ: {} Hz\n\
        - SYSCLK:   {} Hz\n\
        - PCLK1:    {} Hz\n\
        - PCLK2:    {} Hz\n\
        - ADCCLK:   {} Hz\n\
        ADC:\n\
        - RESOLUTION_BITS: {}\n\
        - MAX_POSSIBLE_SAMPLE: {}\n\
        - OVERSAMPLE: {}\n\
        - SAMPLES_PER_SEC_RAW:       {}.{} (oversampled)\n\
        - SAMPLES_PER_SEC_PROCESSED: {}.{}\n\
        - BUFFERS_PER_SEC: {}\n\
        - BUF_LEN_RAW:       {} (oversampled)\n\
        - BUF_LEN_PROCESSED: {}\n\
        FFT:\n\
        - WINDOW: {}\n\
        - BUF_LEN_REAL:         {}\n\
        - BUF_LEN_COMPLEX:      {}\n\
        - BUF_LEN_COMPLEX_REAL: {}\n\
        - FREQ_RESOLUTION: {}.{} Hz\n\
        - MAX_FREQ: {} Hz\n\
        - MAX_AMPLITUDE: {}\n\
        FFT analysis:\n\
        - MAX_PEAKS: {}\n\
        - NOISE_FLOOR_AMPLITUDE: {}\n\
        Indicator LEDs:\n\
        - PWM_FREQ: {} Hz\n\
        Pulse generation:\n\
        - DURATION_RANGE: {}.{} .. {}.{} us\n\
        - SCHEDULING_OFFSET: {}.{} us\n\
        ",
        debug::FAKE_INPUT_DATA,
        debug::FAKE_INPUT_CYCLES_PER_BUF,
        debug::FAKE_INPUT_CYCLES_PER_BUF * adc::BUFFERS_PER_SEC,
        debug::FAKE_INPUT_AMPLITUDE,
        debug::LOG_TIMING,
        debug::LOG_CONTROL_VALUES,
        debug::LOG_LAST_FEW_SAMPLES,
        debug::LOG_LAST_N_SAMPLES,
        debug::LOG_FFT_PEAKS,
        debug::LOG_ALL_FFT_AMPLITUDES,
        debug::LOG_ALL_PULSES,
        clk::HSE_FREQ.to_Hz(),
        clk::SYSCLK.to_Hz(),
        clk::PCLK1.to_Hz(),
        clk::PCLK2.to_Hz(),
        clk::ADCCLK.to_Hz(),
        adc::RESOLUTION_BITS,
        adc::MAX_POSSIBLE_SAMPLE,
        adc::OVERSAMPLE,
        adc::SAMPLES_PER_SEC_RAW_X100 / 100,
        adc::SAMPLES_PER_SEC_RAW_X100 % 100,
        adc::SAMPLES_PER_SEC_PROCESSED_X100 / 100,
        adc::SAMPLES_PER_SEC_PROCESSED_X100 % 100,
        adc::BUFFERS_PER_SEC,
        adc::BUF_LEN_RAW,
        adc::BUF_LEN_PROCESSED,
        fft::WINDOW,
        fft::BUF_LEN_REAL,
        fft::BUF_LEN_COMPLEX,
        fft::BUF_LEN_COMPLEX_REAL,
        fft::FREQ_RESOLUTION_X1000 / 1000,
        fft::FREQ_RESOLUTION_X1000 % 1000,
        fft::MAX_FREQ,
        fft::MAX_AMPLITUDE,
        fft::analysis::MAX_PEAKS,
        fft::analysis::NOISE_FLOOR_AMPLITUDE,
        indicator::PWM_FREQ.to_Hz(),
        pulse::DURATION_RANGE.start.to_nanos() / 1000,
        pulse::DURATION_RANGE.start.to_nanos() % 1000,
        pulse::DURATION_RANGE.end.to_nanos() / 1000,
        pulse::DURATION_RANGE.end.to_nanos() % 1000,
        pulse::SCHEDULING_OFFSET.to_nanos() / 1000,
        pulse::SCHEDULING_OFFSET.to_nanos() % 1000,
    );
}

pub mod debug;

/// Clock configuration
///
/// See clock tree in https://www.st.com/resource/en/datasheet/stm32f103c8.pdf
/// Rough layout:
///
///   SYSCLK -> AHB prescaler -> APB1 prescaler -> PCLK1
///              / 1,2..512   |   / 1,2,4,8,16
///                           |
///                           -> APB2 prescaler -> PCLK2
///                               / 1,2,4,8,16  |
///                                             |
///                                             -> TIM1 prescaler -> TIM1
///                                             |   * if APB2_pre == 1 { 1 } else { 2 }
///                                             |
///                                             -> ADC prescaler -> ADCCLK
///                                                 / 2,4,6,8
pub mod clk {
    use fugit::Hertz;

    /// Use external oscillator (required to get max 72MHz sysclk)
    pub const HSE_FREQ: Hertz<u32> = Hertz::<u32>::MHz(8);

    /// PLLMUL @ x6 (max 72MHz)
    pub const SYSCLK: Hertz<u32> = Hertz::<u32>::MHz(48);
    pub const SYSCLK_HZ: u32 = SYSCLK.to_Hz();

    // For timer outputs, only need >= 1MHz since minimum pulse duration is 1us

    /// APB1 prescaler @ /16 (max 36MHz)
    pub const PCLK1: Hertz<u32> = Hertz::<u32>::MHz(3);
    /// APB2 prescaler @ /16 (max 72MHz)
    pub const PCLK2: Hertz<u32> = Hertz::<u32>::MHz(3);

    /// TIM1 prescaler @ /1
    pub const TIM1CLK: Hertz<u32> = {
        // only accurate if AHB prescaler = 1
        let apb2_prescaler_is_1 = PCLK2.const_eq(SYSCLK);
        if apb2_prescaler_is_1 {
            PCLK2
        } else {
            // * 2
            match PCLK2.checked_add(PCLK2) {
                Some(clk) => clk,
                None => panic!("overflow doubling PCLK2"),
            }
        }
    };
    pub const TIM1CLK_HZ: u32 = TIM1CLK.to_Hz();

    /// ADC prescaler @ /2 (max 14MHz, min 600kHz)
    pub const ADCCLK: Hertz<u32> = Hertz::<u32>::kHz(1500);
}

// Prolog for clock config:
//
//   :- use_module(library(clpfd)).
//   sampleFreq(SYSCLK, AHB_PRE, APB2_PRE, ADC_PRE, ADC_SAMPLE_x10_UNADJUSTED, FREQ) :-
//     SYSCLK #> 0,
//     AHB_PRE #>= 1,
//     AHB_PRE #=< 512,
//     member(APB2_PRE, [1, 2, 4, 8, 16]),
//     member(ADC_PRE, [2, 4, 6, 8]),
//     member(ADC_SAMPLE_x10_UNADJUSTED, [15, 75, 135, 285, 415, 555, 715, 2395]),
//     ADC_SAMPLE_x10 #= ADC_SAMPLE_x10_UNADJUSTED + 125,
//     ADCCLK #= SYSCLK // AHB_PRE // APB2_PRE // ADC_PRE,
//     ADCCLK #>= 600000,
//     ADCCLK #=< 14000000,
//     FREQ #= ADCCLK * 10 // ADC_SAMPLE_x10.
//
// Query:
//
//   sampleFreq(72000000, 1, APB2_PRE, ADC_PRE, ADC_SAMPLE_x10, FREQ), FREQ #>= 40000, FREQ #=< 50000

/// ADC configuration
pub mod adc {
    use crate::config;
    use stm32f1xx_hal::adc::SampleTime;

    /// The resolution of the hardware ADC being used.
    pub const RESOLUTION_BITS: u32 = 12;

    /// The maximum possible sample value from the hardware ADC.
    #[allow(clippy::cast_possible_truncation)]
    pub const MAX_POSSIBLE_SAMPLE: u16 = (1 << RESOLUTION_BITS as u16) - 1;

    /// ADC averages x samples for each data point
    pub const OVERSAMPLE: usize = 2;

    /// Sample at ADCCLK / this
    const SAMPLE_CYC_X10_UNADJUSTED: usize = 285;
    pub const SAMPLE: SampleTime = match SAMPLE_CYC_X10_UNADJUSTED {
        15 => SampleTime::T_1,
        75 => SampleTime::T_7,
        135 => SampleTime::T_13,
        285 => SampleTime::T_28,
        415 => SampleTime::T_41,
        555 => SampleTime::T_55,
        715 => SampleTime::T_71,
        2395 => SampleTime::T_239,
        _ => panic!("Invalid sample cycles"),
    };
    /// The _real_ sample rate, including an additional 12.5 cycles for successive approximation
    /// See ADC characteristics in https://www.st.com/resource/en/datasheet/stm32f103c8.pdf
    const SAMPLE_CYC: usize = (SAMPLE_CYC_X10_UNADJUSTED + 125) / 10;

    /// Number of raw ADC samples, per second (oversampled)
    pub(super) const SAMPLES_PER_SEC_RAW_X100: usize =
        100 * config::clk::ADCCLK.to_Hz() as usize / SAMPLE_CYC;

    pub(super) const SAMPLES_PER_SEC_PROCESSED_X100: usize = SAMPLES_PER_SEC_RAW_X100 / OVERSAMPLE;

    /// Swap buffers ~32 times per second
    /// Note that 1/32 notes (semidemiquavers) at 60 bpm are 1/8 second
    pub(super) const BUFFERS_PER_SEC: usize = 32;

    /// Raw, differential, oversampled samples per buffer.
    ///
    /// Note: this may not result in a perfect number of buffers per second,
    /// since it is unlikely that the sample rate is evenly divisible.
    pub const BUF_LEN_RAW: usize = {
        let approx_len = SAMPLES_PER_SEC_RAW_X100 / BUFFERS_PER_SEC / 100;
        // make divisible by OVERSAMPLE so processed buffer fits in perfectly
        let remainder = approx_len % OVERSAMPLE;
        approx_len - remainder
    };

    /// Processed, single-ended, averaged samples per buffer.
    pub const BUF_LEN_PROCESSED: usize = BUF_LEN_RAW / OVERSAMPLE;

    const _: () = assert!(
        BUF_LEN_PROCESSED * OVERSAMPLE == BUF_LEN_RAW,
        "processed buf len should perfectly divide raw buf len"
    );
}

/// FFT configuration
pub mod fft {
    use crate::config;
    use crate::fft;
    use crate::math::{const_scale_by_u16_u16, ScalingFactor};
    use defmt::Format;

    /// Possible window functions to apply to sampled data before running the FFT.
    ///
    /// Window functions closer to the top have less attenuation and more frequency resolution (sharper peaks),
    /// but more significant sidelobes and noise.
    ///
    /// Amplitudes below are with FAKE_INPUT_CYCLES_PER_BUF=8 and FAKE_INPUT_AMPLITUDE=u16::MAX/2.
    #[allow(dead_code)]
    #[derive(Format)]
    pub enum Window {
        /// Hard-edged rectangle window.
        ///
        /// Provides little attenuation (amplitude 3400).
        /// Provides the sharpest peaks, but with significant ringing.
        /// Generally should not be used except for debugging.
        Rectangle,
        /// Hamming window.
        ///
        /// Provides some attenuation (amplitude 1700).
        /// Provides slightly sharper peaks than Hann, and lower sidelobes, but with slightly more ringing.
        Hamming,
        /// Hann window.
        ///
        /// Provides some attenuation (amplitude 1800).
        Hann,
        /// Blackman window.
        ///
        /// Provides some attenuation (amplitude 1400).
        /// Provides slightly wider peaks than Hamming or Hann, but with very good suppression of sidelobes and ringing.
        Blackman,
    }

    /// Window type for filtering FFT input
    pub const WINDOW: Window = Window::Hamming;

    /// FFT buffer size should be as large as possible for higher resolution
    pub const BUF_LEN_REAL: usize = 2048;

    const _: () = assert!(BUF_LEN_REAL.is_power_of_two());
    const _: () = assert!(BUF_LEN_REAL >= config::adc::BUF_LEN_PROCESSED);

    /// Complex ADC buffer is half the size, since each `Complex<i16>` holds two samples
    pub const BUF_LEN_COMPLEX: usize = BUF_LEN_REAL / 2;

    /// The part of the complex ADC buffer holding real frequencies is half the size,
    /// since imaginary frequencies occupy the other half.
    pub const BUF_LEN_COMPLEX_REAL: usize = BUF_LEN_COMPLEX / 2;

    /// Each FFT bin is this many Hz apart
    pub const FREQ_RESOLUTION_X1000: usize =
        10 * config::adc::SAMPLES_PER_SEC_PROCESSED_X100 / BUF_LEN_REAL;

    /// Frequency of the maximum FFT bin
    pub const MAX_FREQ: u16 = (FREQ_RESOLUTION_X1000 * BUF_LEN_COMPLEX_REAL / 1000) as u16;

    /// Maximum feasible amplitude of an FFT peak.
    pub const MAX_AMPLITUDE: u16 = {
        // samples are scaled up to full i16 range, allowing a potential amplitude of all 16 bits
        let amplitude = u16::MAX;
        // amplitude is scaled down by zeroed padding added to samples
        #[allow(clippy::cast_possible_truncation)]
        let zeroed_padding_factor =
            ScalingFactor::from_ratio(config::adc::BUF_LEN_PROCESSED as u16, BUF_LEN_REAL as u16);
        let amplitude = const_scale_by_u16_u16(amplitude, zeroed_padding_factor);
        // amplitude is scaled down by window function
        let window_factor = fft::window::amplitude_scale_factor();
        let amplitude = const_scale_by_u16_u16(amplitude, window_factor);
        // for some unexplainable reason, the actual achievable amplitude is a factor of slightly less than 3 off...
        // use a factor of approximately 2*sqrt(2) to provide some safety margin
        let fudge_factor = ScalingFactor::from_ratio(1000, 2828);
        let amplitude = const_scale_by_u16_u16(amplitude, fudge_factor);
        amplitude
    };

    pub mod analysis {
        /// Maximum number of peaks to find in the FFT spectrum.
        pub const MAX_PEAKS: usize = 8;

        /// Min amplitude for a FFT bin to be considered a peak.
        /// In addition to this threshold, another threshold is applied in proportion to the amplitude of the highest peak.
        pub const NOISE_FLOOR_AMPLITUDE: u16 = 100;
        pub const NOISE_FLOOR_AMPLITUDE_SQUARED: u32 = (NOISE_FLOOR_AMPLITUDE as u32).pow(2);
    }
}

/// Indicator LED configuration
pub mod indicator {
    use fugit::Hertz;

    pub const PWM_FREQ: Hertz<u32> = Hertz::<u32>::kHz(100);
}

/// Pulse generation configuration
pub mod pulse {
    use crate::time::{Duration, PulseDuration};
    use core::ops::Range;

    /// Pulse duration range when control is set to minimum/maximum
    pub const DURATION_RANGE: Range<PulseDuration> =
        PulseDuration::micros(1)..PulseDuration::micros(10);

    /// Start scheduling pulses this far in the future.
    ///
    /// This ensures that we don't try to schedule a pulse, e.g., just 1 tick after the current time,
    /// causing us to miss the deadline (and wait until the timer wraps).
    ///
    /// It also provides a minimum repeat rate, for the same reason.
    pub const SCHEDULING_OFFSET: Duration = Duration::micros(50);
}
