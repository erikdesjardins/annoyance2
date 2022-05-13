#![no_main]
#![no_std]

use annoyance2 as _; // global logger + panicking-behavior + memory layout

#[rtic::app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use dwt_systick_monotonic::{fugit::RateExtU32, DwtSystick};
    use stm32f1xx_hal::prelude::*;

    #[monotonic(binds = SysTick, default = true)]
    type DwtMono = DwtSystick<72_000_000>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {}

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("start");

        // Setup clocks
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
        //
        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();
        let clocks = rcc
            .cfgr
            .use_hse(8.MHz()) // use external oscillator (required to get max 72MHz sysclk)
            .sysclk(72.MHz()) // PLLMUL @ x9 (max 72MHz)
            // for timer outputs, only need >= 1MHz since minimum pulse duration is 1us
            .pclk1(9.MHz()) // APB1 prescaler @ /8 (max 36MHz)
            .pclk2(9.MHz()) // APB2 prescaler @ /8 (max 72MHz)
            // for adc, want as low as possible since we're sampling audio at 48kHz
            .adcclk(1125.kHz()) // ADC prescaler @ /8 (max 14MHz, min 600kHz)
            .freeze(&mut flash.acr);

        defmt::info!("configured clocks");

        let mono = DwtSystick::new(
            &mut cx.core.DCB,
            cx.core.DWT,
            cx.core.SYST,
            clocks.sysclk().to_Hz(),
        );

        task1::spawn().ok();

        (Shared {}, Local {}, init::Monotonics(mono))
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        defmt::info!("idle");

        loop {
            // Note that using `wfi` here breaks debugging,
            // so if desired we should only do that in release mode.
            continue;
        }
    }

    #[task]
    fn task1(_cx: task1::Context) {
        defmt::info!("Hello from task1!");
    }
}
