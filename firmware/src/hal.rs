//! Extensions to the `stm32f1xx-hal` Hardware Abstraction Layer.

pub mod tim;

#[allow(non_camel_case_types)]
pub mod pins {
    use stm32f1xx_hal::gpio::{Alternate, Analog, Output, Pin, PushPull};

    /// Amplified single-ended audio input
    pub type A0_ADC1C0 = Pin<'A', 0, Analog>;

    /// Threshold potentiometer
    pub type A2_ADC2C2 = Pin<'A', 2, Analog>;

    /// Pulse width potentiometer
    pub type A1_ADC2C1 = Pin<'A', 1, Analog>;

    /// Amplitude indicator LED 1
    pub type B1_TIM3C4 = Pin<'B', 1, Alternate<PushPull>>;
    /// Amplitude indicator LED 2
    pub type B0_TIM3C3 = Pin<'B', 0, Alternate<PushPull>>;
    /// Amplitude indicator LED 3
    pub type A7_TIM3C2 = Pin<'A', 7, Alternate<PushPull>>;
    /// Amplitude indicator LED 4
    pub type A6_TIM3C1 = Pin<'A', 6, Alternate<PushPull>>;

    /// Above threshold indicator LED 1
    pub type B6_TIM4C1 = Pin<'B', 6, Alternate<PushPull>>;
    /// Above threshold indicator LED 2
    pub type B7_TIM4C2 = Pin<'B', 7, Alternate<PushPull>>;
    /// Above threshold indicator LED 3
    pub type B8_TIM4C3 = Pin<'B', 8, Alternate<PushPull>>;
    /// Above threshold indicator LED 4
    pub type B9_TIM4C4 = Pin<'B', 9, Alternate<PushPull>>;

    /// Pulse output
    pub type A8_TIM1C1_PULSE = Pin<'A', 8, Alternate<PushPull>>;

    /// Debug LED output
    pub type C13_DEBUG_LED = Pin<'C', 13, Output<PushPull>>;
}
