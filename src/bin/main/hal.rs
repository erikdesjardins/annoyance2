//! Extensions to the `stm32f1xx-hal` Hardware Abstraction Layer.

pub mod tim;

#[allow(non_camel_case_types)]
pub mod pins {
    use stm32f1xx_hal::gpio::{Alternate, Analog, Output, Pin, PushPull, CRH, CRL};

    pub type A0_ADC1C0 = Pin<Analog, CRL, 'A', 0>;
    pub type A1_ADC2C1 = Pin<Analog, CRL, 'A', 1>;
    pub type A2_ADC2C2 = Pin<Analog, CRL, 'A', 2>;
    pub type A8_TIM1C1_PULSE = Pin<Alternate<PushPull>, CRH, 'A', 8>;
    pub type C13_DEBUG_LED = Pin<Output<PushPull>, CRH, 'C', 13>;
}
