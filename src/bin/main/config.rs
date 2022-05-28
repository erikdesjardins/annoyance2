pub fn dump_to_log() {
    defmt::info!(
        "\n\
        Debugging flags:\n\
        - FAKE_INPUT_DATA: {}\n\
        - FAKE_INPUT_CYCLES_PER_BUF: {}\n\
        - FAKE_INPUT_AMPLITUDE: {}\n\
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
        - FREQ_RESOLUTION: {}.{} Hz\n\
        ",
        debug::FAKE_INPUT_DATA,
        debug::FAKE_INPUT_CYCLES_PER_BUF,
        debug::FAKE_INPUT_AMPLITUDE,
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
    );
}

/// Debugging flags
pub mod debug {
    pub const FAKE_INPUT_DATA: bool = false;
    pub const FAKE_INPUT_CYCLES_PER_BUF: usize = 8; // frequency = this * BUFFERS_PER_SEC
    pub const FAKE_INPUT_AMPLITUDE: u16 = u16::MAX / 2;
}

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
///                                             -> ADC prescaler -> ADCCLK
///                                                 / 2,4,6,8
pub mod clk {
    use fugit::Rate;

    /// Use external oscillator (required to get max 72MHz sysclk)
    pub const HSE_FREQ: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::MHz(8);

    /// PLLMUL @ x9 (max 72MHz)
    pub const SYSCLK: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::MHz(72);
    pub const SYSCLK_HZ: u32 = SYSCLK.to_Hz();

    // For timer outputs, only need >= 1MHz since minimum pulse duration is 1us

    /// APB1 prescaler @ /8 (max 36MHz)
    pub const PCLK1: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::MHz(9);
    /// APB2 prescaler @ /8 (max 72MHz)
    pub const PCLK2: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::MHz(9);

    // For adc, want slow enough to sample audio, but fast enough that register writes are acknowledged fast (?)

    /// ADC prescaler @ /4 (max 14MHz, min 600kHz)
    pub const ADCCLK: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::kHz(2250);
}

// Prolog for clock config:
//
//   :- use_module(library(clpfd)).
//   sampleFreq(SYSCLK, AHB_PRE, APB2_PRE, ADC_PRE, ADC_SAMPLE_x10, FREQ) :-
//     SYSCLK #> 0,
//     AHB_PRE #>= 1,
//     AHB_PRE #=< 512,
//     member(APB2_PRE, [1, 2, 4, 8, 16]),
//     member(ADC_PRE, [2, 4, 6, 8]),
//     member(ADC_SAMPLE_x10, [15, 75, 135, 285, 415, 555, 715, 2395]),
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

    /// ADC averages 1 samples for each data point
    pub const OVERSAMPLE: usize = 1;

    /// Sample at ADCCLK / 55.5 = 40540 Hz (~20kHz per channel)
    const SAMPLE_CYC_X10: usize = 555;
    pub const SAMPLE: SampleTime = match SAMPLE_CYC_X10 {
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

    /// Number of raw ADC samples, per second (both channels, oversampled)
    pub(super) const SAMPLES_PER_SEC_RAW_X100: usize =
        100 * config::clk::ADCCLK.to_Hz() as usize * 10 / SAMPLE_CYC_X10;

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

    #[allow(dead_code)]
    #[derive(Format)]
    pub enum Window {
        Rectangle,
        Hamming,
        BlackmanNutall,
        BlackmanHarris,
    }

    /// Window type for filtering FFT input
    pub const WINDOW: Window = Window::BlackmanHarris;

    /// FFT buffer size should be as large as possible for higher resolution (~20Hz in this case)
    pub const BUF_LEN_REAL: usize = 4096;
    const _: () = assert!(BUF_LEN_REAL.is_power_of_two());
    const _: () = assert!(BUF_LEN_REAL >= config::adc::BUF_LEN_PROCESSED);

    /// Complex ADC buffer is half the size, since each `Complex<i16>` holds two samples
    pub const BUF_LEN_COMPLEX: usize = BUF_LEN_REAL / 2;

    /// Each FFT bin is this many Hz apart
    pub const FREQ_RESOLUTION_X1000: usize =
        10 * config::adc::SAMPLES_PER_SEC_PROCESSED_X100 / BUF_LEN_COMPLEX;
}
