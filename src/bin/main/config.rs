/// Debugging flags
pub mod debug {
    pub const LOG_FINAL_ADC_SAMPLES: bool = true;

    pub const LOG_FFT_STATS: bool = true;
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

    /// ADC prescaler @ /8 (max 14MHz, min 600kHz)
    pub const ADCCLK: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::kHz(1125);
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
    const CHANNELS: u32 = 2;

    /// Sample at ADCCLK / 13.5 = 83333 Hz (~40kHz per channel)
    const SAMPLE_CYC_X10: u32 = 135;
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

    /// Number of ADC samples, per second, per channel
    pub const SAMPLES_PER_SEC_PER_CHANNEL: u32 =
        config::clk::ADCCLK.to_Hz() * 10 / SAMPLE_CYC_X10 / CHANNELS;

    /// Swap buffers ~32 times per second
    /// Note that 1/32 notes (semidemiquavers) at 60 bpm are 1/8 second
    const BUFFERS_PER_SEC: usize = 32;

    pub const BUF_LEN_PER_CHANNEL: usize = SAMPLES_PER_SEC_PER_CHANNEL as usize / BUFFERS_PER_SEC;
}

/// FFT configuration
pub mod fft {
    use crate::config;

    #[allow(dead_code)]
    pub enum Window {
        Rectangle,
        BlackmanHarris,
    }

    /// Window type for filtering FFT input
    pub const WINDOW: Window = Window::BlackmanHarris;

    /// FFT buffer size should be as large as possible for higher resolution (~20Hz in this case)
    pub const BUF_LEN_REAL: usize = 4096;
    const _: () = assert!(BUF_LEN_REAL.is_power_of_two());
    /// Complex ADC buffer is half the size, since each `Complex<i16>` holds two samples
    pub const BUF_LEN_COMPLEX: usize = BUF_LEN_REAL / 2;

    /// Each FFT bin is this many Hz apart
    pub const FREQ_RESOLUTION_X1000: usize =
        config::adc::SAMPLES_PER_SEC_PER_CHANNEL as usize * 1000 / BUF_LEN_COMPLEX;
}
