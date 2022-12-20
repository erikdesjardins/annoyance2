//! Extensions to the `stm32f1xx-hal` Hardware Abstraction Layer.

pub mod tim;

#[allow(non_camel_case_types)]
pub mod pins {
    use stm32f1xx_hal::gpio::{Alternate, Analog, Output, Pin, PushPull, CRH, CRL};

    /// Amplified single-ended audio input
    pub type A0_ADC1C0 = Pin<Analog, CRL, 'A', 0>;

    /// Threshold potentiometer
    pub type A2_ADC2C2 = Pin<Analog, CRL, 'A', 2>;

    /// Pulse width potentiometer
    pub type A1_ADC2C1 = Pin<Analog, CRL, 'A', 1>;

    /// Amplitude indicator LED 1
    pub type B1_TIM3C4 = Pin<Alternate<PushPull>, CRL, 'B', 1>;
    /// Amplitude indicator LED 2
    pub type B0_TIM3C3 = Pin<Alternate<PushPull>, CRL, 'B', 0>;
    /// Amplitude indicator LED 3
    pub type A7_TIM3C2 = Pin<Alternate<PushPull>, CRL, 'A', 7>;
    /// Amplitude indicator LED 4
    pub type A6_TIM3C1 = Pin<Alternate<PushPull>, CRL, 'A', 6>;

    /// Above threshold indicator LED 1
    pub type B6_TIM4C1 = Pin<Alternate<PushPull>, CRL, 'B', 6>;
    /// Above threshold indicator LED 2
    pub type B7_TIM4C2 = Pin<Alternate<PushPull>, CRL, 'B', 7>;
    /// Above threshold indicator LED 3
    pub type B8_TIM4C3 = Pin<Alternate<PushPull>, CRH, 'B', 8>;
    /// Above threshold indicator LED 4
    pub type B9_TIM4C4 = Pin<Alternate<PushPull>, CRH, 'B', 9>;

    /// Pulse output
    pub type A8_TIM1C1_PULSE = Pin<Alternate<PushPull>, CRH, 'A', 8>;

    /// Debug LED output
    pub type C13_DEBUG_LED = Pin<Output<PushPull>, CRH, 'C', 13>;
}
