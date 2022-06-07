pub fn dump_to_log() {
    defmt::info!(
        "\n\
        Debugging flags:\n\
        - FAKE_INPUT_DATA:           {}\n\
        - FAKE_INPUT_CYCLES_PER_BUF: {} ({} Hz)\n\
        - FAKE_INPUT_AMPLITUDE:      {}\n\
        - LOG_LAST_FEW_SAMPLES: {}\n\
        - LOG_LAST_N_SAMPLES:   {}\n\
        - LOG_FFT_PEAKS: {}\n\
        - LOG_ALL_FFT_AMPLITUDES: {}\n\
        Clocks:\n\
        - HSE_FREQ: {} Hz\n\
        - SYSCLK:   {} Hz\n\
        - PCLK1:    {} Hz\n\
        - PCLK2:    {} Hz\n\
        - ADCCLK:   {} Hz\n\
        ADC:\n\
        - CHANNELS: {}\n\
        - OVERSAMPLE: {}\n\
        - SAMPLES_PER_SEC_RAW:       {}.{} (all channels, oversampled)\n\
        - SAMPLES_PER_SEC_PROCESSED: {}.{} (one channel)\n\
        - BUFFERS_PER_SEC: {}\n\
        - BUF_LEN_RAW:       {} (all channels, oversampled)\n\
        - BUF_LEN_PROCESSED: {} (one channel)\n\
        FFT:\n\
        - WINDOW: {}\n\
        - BUF_LEN_REAL:    {}\n\
        - BUF_LEN_COMPLEX: {}\n\
        - FREQ_RESOLUTION: {}.{} Hz (max {}.{} Hz)\n\
        FFT analysis:\n\
        - MAX_PEAKS: {}\n\
        - AMPLITUDE_THRESHOLD: {}\n\
        Pulse generation:\n\
        - DURATION: {}.{} us\n\
        ",
        debug::FAKE_INPUT_DATA,
        debug::FAKE_INPUT_CYCLES_PER_BUF,
        debug::FAKE_INPUT_CYCLES_PER_BUF * adc::BUFFERS_PER_SEC,
        debug::FAKE_INPUT_AMPLITUDE,
        debug::LOG_LAST_FEW_SAMPLES,
        debug::LOG_LAST_N_SAMPLES,
        debug::LOG_FFT_PEAKS,
        debug::LOG_ALL_FFT_AMPLITUDES,
        clk::HSE_FREQ.to_Hz(),
        clk::SYSCLK.to_Hz(),
        clk::PCLK1.to_Hz(),
        clk::PCLK2.to_Hz(),
        clk::ADCCLK.to_Hz(),
        adc::CHANNELS,
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
        fft::FREQ_RESOLUTION_X1000 / 1000,
        fft::FREQ_RESOLUTION_X1000 % 1000,
        fft::FREQ_RESOLUTION_X1000 * fft::BUF_LEN_COMPLEX / 2 / 1000,
        fft::FREQ_RESOLUTION_X1000 * fft::BUF_LEN_COMPLEX / 2 % 1000,
        fft::analysis::MAX_PEAKS,
        fft::analysis::AMPLITUDE_THRESHOLD,
        pulse::DURATION.to_nanos() / 1000,
        pulse::DURATION.to_nanos() % 1000,
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
///                                             |   if APB2_pre == 1 { 1 } else { 2 }
///                                             |
///                                             -> ADC prescaler -> ADCCLK
///                                                 / 2,4,6,8
pub mod clk {
    use fugit::Hertz;

    /// Use external oscillator (required to get max 72MHz sysclk)
    pub const HSE_FREQ: Hertz<u32> = Hertz::<u32>::MHz(8);

    /// PLLMUL @ x9 (max 72MHz)
    pub const SYSCLK: Hertz<u32> = Hertz::<u32>::MHz(72);
    pub const SYSCLK_HZ: u32 = SYSCLK.to_Hz();

    // For timer outputs, only need >= 1MHz since minimum pulse duration is 1us

    /// APB1 prescaler @ /8 (max 36MHz)
    pub const PCLK1: Hertz<u32> = Hertz::<u32>::MHz(9);
    /// APB2 prescaler @ /8 (max 72MHz)
    pub const PCLK2: Hertz<u32> = Hertz::<u32>::MHz(9);

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

    // For adc, want slow enough to sample audio, but fast enough that register writes are acknowledged fast (?)

    /// ADC prescaler @ /2 (max 14MHz, min 600kHz)
    pub const ADCCLK: Hertz<u32> = Hertz::<u32>::kHz(4500);
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

    /// ADC scans two channels for differential input
    pub const CHANNELS: usize = 2;

    /// ADC averages x samples for each data point
    pub const OVERSAMPLE: usize = 2;

    /// Sample at ADCCLK / this
    const SAMPLE_CYC_X10_UNADJUSTED: usize = 415;
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

    /// Number of raw ADC samples, per second (both channels, oversampled)
    pub(super) const SAMPLES_PER_SEC_RAW_X100: usize =
        100 * config::clk::ADCCLK.to_Hz() as usize / SAMPLE_CYC;

    pub(super) const SAMPLES_PER_SEC_PROCESSED_X100: usize =
        SAMPLES_PER_SEC_RAW_X100 / CHANNELS / OVERSAMPLE;

    /// Swap buffers ~32 times per second
    /// Note that 1/32 notes (semidemiquavers) at 60 bpm are 1/8 second
    pub(super) const BUFFERS_PER_SEC: usize = 32;

    /// Raw, differential, oversampled samples per buffer.
    ///
    /// Note: this may not result in a perfect number of buffers per second,
    /// since it is unlikely that the sample rate is evenly divisible.
    pub const BUF_LEN_RAW: usize = {
        let approx_len = SAMPLES_PER_SEC_RAW_X100 / BUFFERS_PER_SEC / 100;
        // make divisible by CHANNELS and OVERSAMPLE so processed buffer fits in perfectly
        let remainder = approx_len % (CHANNELS * OVERSAMPLE);
        approx_len - remainder
    };

    /// Processed, single-ended, averaged samples per buffer.
    pub const BUF_LEN_PROCESSED: usize = BUF_LEN_RAW / CHANNELS / OVERSAMPLE;

    const _: () = assert!(
        BUF_LEN_PROCESSED * CHANNELS * OVERSAMPLE == BUF_LEN_RAW,
        "processed buf len should perfectly divide raw buf len"
    );
}

/// FFT configuration
pub mod fft {
    use crate::config;
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

    /// Each FFT bin is this many Hz apart
    pub const FREQ_RESOLUTION_X1000: usize =
        10 * config::adc::SAMPLES_PER_SEC_PROCESSED_X100 / BUF_LEN_REAL;

    pub mod analysis {
        /// Maximum number of peaks to find in the FFT spectrum
        pub const MAX_PEAKS: usize = 8;

        /// Minimum amplitude for a FFT bin to be considered a peak
        pub(in crate::config) const AMPLITUDE_THRESHOLD: u16 = 50;
        pub const AMPLITUDE_THRESHOLD_SQUARED: u32 = (AMPLITUDE_THRESHOLD as u32).pow(2);
    }
}

/// Pulse generation configuration
pub mod pulse {
    use crate::config;
    use fugit::Duration;

    /// Pulse duration
    pub const DURATION: Duration<u32, 1, { config::clk::TIM1CLK_HZ }> =
        Duration::<u32, 1, { config::clk::TIM1CLK_HZ }>::micros(1);
}
