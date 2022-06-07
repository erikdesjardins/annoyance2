//! Extensions to the `stm32f1xx-hal` Hardware Abstraction Layer.

use crate::config;
use embedded_hal::adc::Channel;
use stm32f1xx_hal::adc::{Adc, SetChannels};
use stm32f1xx_hal::device::ADC1;
use stm32f1xx_hal::prelude::*;

pub mod tim;

#[allow(non_camel_case_types)]
pub mod pins {
    use stm32f1xx_hal::gpio::{Alternate, Analog, Output, Pin, PushPull, CRH, CRL};

    pub type A0_ADC1C0 = Pin<Analog, CRL, 'A', 0>;
    pub type A4_ADC1C4 = Pin<Analog, CRL, 'A', 4>;
    pub type A2_PWM_VIRT_GND = Pin<Alternate<PushPull>, CRL, 'A', 2>;
    pub type A8_TIM1C1_PULSE = Pin<Alternate<PushPull>, CRH, 'A', 8>;
    pub type C13_DEBUG_LED = Pin<Output<PushPull>, CRH, 'C', 13>;
}

/// Holds the two ADC pins which are alternately sampled for differential audio input.
pub struct AdcPins(pub pins::A0_ADC1C0, pub pins::A4_ADC1C4);

impl AdcPins {
    fn channels() -> [u8; 2] {
        [
            <pins::A0_ADC1C0 as Channel<ADC1>>::channel(),
            <pins::A4_ADC1C4 as Channel<ADC1>>::channel(),
        ]
    }
}

impl SetChannels<AdcPins> for Adc<ADC1> {
    fn set_samples(&mut self) {
        for channel in AdcPins::channels() {
            self.set_channel_sample_time(channel, config::adc::SAMPLE);
        }
    }
    fn set_sequence(&mut self) {
        self.set_regular_sequence(&AdcPins::channels());
        self.set_continuous_mode(true);
    }
}
