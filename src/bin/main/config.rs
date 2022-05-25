use fugit::Rate;
use stm32f1xx_hal::adc::SampleTime;

// See clock tree in https://www.st.com/resource/en/datasheet/stm32f103c8.pdf
// Rough layout:
//
//   SYSCLK -> AHB prescaler -> APB1 prescaler -> PCLK1
//              / 1,2..512   |   / 1,2,4,8,16
//                           |
//                           -> APB2 prescaler -> PCLK2
//                               / 1,2,4,8,16  |
//                                             |
//                                             -> ADC prescaler -> ADCCLK
//                                                 / 2,4,6,8

// Use external oscillator (required to get max 72MHz sysclk)
pub const HSE_FREQ: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::MHz(8);

// PLLMUL @ x9 (max 72MHz)
pub const SYSCLK: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::MHz(72);
pub const SYSCLK_HZ: u32 = SYSCLK.to_Hz();

// For timer outputs, only need >= 1MHz since minimum pulse duration is 1us
// APB1 prescaler @ /8 (max 36MHz)
pub const PCLK1: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::MHz(9);
// APB2 prescaler @ /8 (max 72MHz)
pub const PCLK2: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::MHz(9);

// For adc, want slow enough to sample audio, but fast enough that register writes are acknowledged fast (?)
// ADC prescaler @ /2 (max 14MHz, min 600kHz)
pub const ADCCLK: Rate<u32, 1, 1> = Rate::<u32, 1, 1>::kHz(4500);

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

// ADC scans two channels for differential input
pub const ADC_CHANNELS: usize = 2;

// Sample at ADCCLK / 55.5 = 81081 Hz (~40kHz per channel)
pub const ADC_SAMPLE_CYC_X10: u32 = 555;
pub const ADC_SAMPLE: SampleTime = match ADC_SAMPLE_CYC_X10 {
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
pub const ADC_SAMPLE_PER_SEC: u32 = ADCCLK.to_Hz() * 10 / ADC_SAMPLE_CYC_X10;

// Swap buffers ~32 times per second, to be able to play 1/32 notes
pub const ADC_BUFFERS_PER_SEC: usize = 32;

pub const ADC_BUF_LEN_PER_CHANNEL: usize =
    ADC_SAMPLE_PER_SEC as usize / ADC_BUFFERS_PER_SEC / ADC_CHANNELS;

#[allow(dead_code)]
pub enum Window {
    Rectangle,
    BlackmanHarris,
}

// Scale up ADC buffer to next power of 2, since that's required for Radix-2 algorithm
pub const FFT_BUF_LEN: usize = ADC_BUF_LEN_PER_CHANNEL.next_power_of_two();
pub const FFT_BUF_LEN_LOG2: usize = usize::BITS as usize - 1 - FFT_BUF_LEN.leading_zeros() as usize;

pub const FFT_WINDOW: Window = Window::BlackmanHarris;

// Each FFT bin is this many Hz apart
pub const FFT_FREQ_RESOLUTION_X1000: u32 = ADC_SAMPLE_PER_SEC * 1000 / FFT_BUF_LEN as u32;
