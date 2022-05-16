#![no_main]
#![no_std]
#![allow(clippy::type_complexity)]

use annoyance2 as _; // global logger + panicking-behavior + memory layout

mod config;
mod fft;

#[rtic::app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [USART1])]
mod app {
    use crate::{config, fft};
    use cortex_m::singleton;
    use dwt_systick_monotonic::DwtSystick;
    use stm32f1xx_hal::adc::{Adc, AdcDma, Continuous};
    use stm32f1xx_hal::device::ADC1;
    use stm32f1xx_hal::dma::{dma1, CircBuffer, Event};
    use stm32f1xx_hal::gpio::{Analog, Output, Pin, PinState, PushPull, CRL, PC13};
    use stm32f1xx_hal::prelude::*;

    #[monotonic(binds = SysTick, default = true)]
    type DwtMono = DwtSystick<{ config::SYSCLK_HZ }>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        adc_dma_buf: CircBuffer<
            [u16; config::ADC_BUF_LEN],
            AdcDma<ADC1, Pin<Analog, CRL, 'A', 0>, Continuous, dma1::C1>,
        >,
        _debug_led: PC13<Output<PushPull>>,
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("Configuring clocks...");

        let mut flash = cx.device.FLASH.constrain();
        let rcc = cx.device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(config::HSE_FREQ)
            .sysclk(config::SYSCLK)
            .pclk1(config::PCLK1)
            .pclk2(config::PCLK2)
            .adcclk(config::ADCCLK)
            .freeze(&mut flash.acr);

        assert!(config::SYSCLK == clocks.sysclk());
        assert!(config::PCLK1 == clocks.pclk1());
        assert!(config::PCLK2 == clocks.pclk2());
        assert!(config::ADCCLK == clocks.adcclk());

        defmt::info!("Configuring ADC DMA transfer...");

        let dma1 = cx.device.DMA1.split();
        let mut dma1_ch1 = dma1.1;
        // Enable interrupts on DMA1_CHANNEL1
        dma1_ch1.listen(Event::HalfTransfer);
        dma1_ch1.listen(Event::TransferComplete);

        let mut adc1 = Adc::adc1(cx.device.ADC1, clocks);
        adc1.set_sample_time(config::ADC_SAMPLE);

        let mut gpioa = cx.device.GPIOA.split();
        let adc_ch0 = gpioa.pa0.into_analog(&mut gpioa.crl);

        let adc_dma = adc1.with_dma(adc_ch0, dma1_ch1);

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

        defmt::info!("Starting ADC DMA transfer...");

        let buf =
            singleton!(: [[u16; config::ADC_BUF_LEN]; 2] = [[0; config::ADC_BUF_LEN]; 2]).unwrap();
        let adc_dma_buf = adc_dma.circ_read(buf);

        defmt::info!("Finished init.");

        (
            Shared {},
            Local {
                adc_dma_buf,
                _debug_led: led,
            },
            init::Monotonics(mono),
        )
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            // Note that using `wfi` here breaks debugging,
            // so if desired we should only do that in release mode.
            continue;
        }
    }

    #[task(binds = DMA1_CHANNEL1, local = [adc_dma_buf], priority = 1)]
    fn adc_dma(cx: adc_dma::Context) {
        defmt::info!("Started processing ADC buffer...");

        let start = monotonics::now();

        let res = cx.local.adc_dma_buf.peek(|half, _| fft::process(half));

        let duration = monotonics::now() - start;

        match res {
            Ok((min, max)) => defmt::info!(
                "Finished processing ADC buffer after {}us. ({} to {})",
                duration.to_micros(),
                min,
                max
            ),
            Err(_) => defmt::warn!(
                "ADC buffer processing did not complete in time (took {}us).",
                duration.to_micros()
            ),
        }
    }
}
