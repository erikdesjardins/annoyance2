#![no_main]
#![no_std]

use annoyance2 as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use dwt_systick_monotonic::fugit::RateExtU32;
    use dwt_systick_monotonic::DwtSystick;
    use stm32f1xx_hal::gpio::{Output, PinState, PushPull, PC13};
    use stm32f1xx_hal::prelude::*;

    #[monotonic(binds = SysTick, default = true)]
    type DwtMono = DwtSystick<72_000_000>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        _debug_led: PC13<Output<PushPull>>,
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("Configuring clocks...");

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

        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();

        // PLLMUL @ x9 (max 72MHz)
        let sysclk = 72.MHz();

        // for timer outputs, only need >= 1MHz since minimum pulse duration is 1us
        // APB1 prescaler @ /8 (max 36MHz)
        let pclk1 = 9.MHz();
        // APB2 prescaler @ /8 (max 72MHz)
        let pclk2 = 9.MHz();

        // for adc, want as low as possible since we're sampling audio at 48kHz
        // ADC prescaler @ /8 (max 14MHz, min 600kHz)
        let adcclk = 1125.kHz();

        let clocks = rcc
            .cfgr
            .use_hse(8.MHz()) // use external oscillator (required to get max 72MHz sysclk)
            .sysclk(sysclk)
            .pclk1(pclk1)
            .pclk2(pclk2)
            .adcclk(adcclk)
            .freeze(&mut flash.acr);

        assert!(sysclk == clocks.sysclk());
        assert!(pclk1 == clocks.pclk1());
        assert!(pclk2 == clocks.pclk2());
        assert!(adcclk == clocks.adcclk());

        defmt::info!("Configuring monotonic timer...");

        let mono = DwtSystick::new(
            &mut cx.core.DCB,
            cx.core.DWT,
            cx.core.SYST,
            clocks.sysclk().to_Hz(),
        );

        defmt::info!("Configuring debug indicator LED...");

        let mut gpioc = cx.device.GPIOC.split();
        let led = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, PinState::Low);

        (Shared {}, Local { _debug_led: led }, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("Reached idle.");

        loop {
            // Note that using `wfi` here breaks debugging,
            // so if desired we should only do that in release mode.
            continue;
        }
    }
}
