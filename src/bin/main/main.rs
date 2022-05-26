#![no_main]
#![no_std]
#![allow(clippy::type_complexity, clippy::needless_range_loop)]
#![warn(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::ptr_as_ptr
)]

use annoyance2 as _; // global logger + panicking-behavior + memory layout

mod adc;
mod config;
mod fixed;

#[rtic::app(device = stm32f1xx_hal::pac, peripherals = true, dispatchers = [USART1])]

mod app {
    use crate::{adc, config};
    use cortex_m::singleton;
    use dwt_systick_monotonic::DwtSystick;
    use embedded_hal::adc::Channel;
    use stm32f1xx_hal::adc::{Adc, AdcDma, ChannelTimeSequence, Scan, SetChannels};
    use stm32f1xx_hal::device::ADC1;
    use stm32f1xx_hal::dma::{dma1, CircBuffer, Event};
    use stm32f1xx_hal::gpio::PinState;
    use stm32f1xx_hal::prelude::*;
    use stm32f1xx_hal::timer::Tim2NoRemap;

    #[monotonic(binds = SysTick, default = true)]
    type DwtMono = DwtSystick<{ config::clk::SYSCLK_HZ }>;

    #[allow(non_camel_case_types)]
    mod pins {
        use stm32f1xx_hal::gpio::{Alternate, Analog, Output, Pin, PushPull, CRH, CRL};

        pub type A0_ADC1C0 = Pin<Analog, CRL, 'A', 0>;
        pub type A1_ADC1C1 = Pin<Analog, CRL, 'A', 1>;
        pub type A2_PWM_VIRT_GND = Pin<Alternate<PushPull>, CRL, 'A', 2>;
        // pub type A8_TIM1C1_PULSE = Pin<Digital, CRL, 'A', 8>;
        pub type C13_DEBUG_LED = Pin<Output<PushPull>, CRH, 'C', 13>;
    }

    pub struct AdcPins(pins::A0_ADC1C0, pins::A1_ADC1C1);

    impl AdcPins {
        fn channels() -> [u8; 2] {
            [
                <pins::A0_ADC1C0 as Channel<ADC1>>::channel(),
                <pins::A1_ADC1C1 as Channel<ADC1>>::channel(),
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

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        adc_dma_transfer: CircBuffer<
            [u16; config::adc::BUF_LEN_PER_CHANNEL * 2],
            AdcDma<ADC1, AdcPins, Scan, dma1::C1>,
        >,
        fft_buf: &'static mut [i16; config::fft::BUF_LEN],
        debug_led: pins::C13_DEBUG_LED,
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("Starting init...");

        let mut afio = cx.device.AFIO.constrain();
        let dma1 = cx.device.DMA1.split();
        let mut flash = cx.device.FLASH.constrain();
        let mut gpioa = cx.device.GPIOA.split();
        let mut gpioc = cx.device.GPIOC.split();
        let rcc = cx.device.RCC.constrain();

        defmt::info!("Configuring clocks...");

        let clocks = rcc
            .cfgr
            .use_hse(config::clk::HSE_FREQ)
            .sysclk(config::clk::SYSCLK)
            .pclk1(config::clk::PCLK1)
            .pclk2(config::clk::PCLK2)
            .adcclk(config::clk::ADCCLK)
            .freeze(&mut flash.acr);

        assert!(config::clk::SYSCLK == clocks.sysclk());
        assert!(config::clk::PCLK1 == clocks.pclk1());
        assert!(config::clk::PCLK2 == clocks.pclk2());
        assert!(config::clk::ADCCLK == clocks.adcclk());

        defmt::info!("Configuring ADC DMA transfer...");

        let mut dma1_ch1 = dma1.1;
        // Enable interrupts on DMA1_CHANNEL1
        dma1_ch1.listen(Event::HalfTransfer);
        dma1_ch1.listen(Event::TransferComplete);

        let mut adc1 = Adc::adc1(cx.device.ADC1, clocks);
        adc1.set_sample_time(config::adc::SAMPLE);

        let adc_ch0: pins::A0_ADC1C0 = gpioa.pa0.into_analog(&mut gpioa.crl);
        let adc_ch1: pins::A1_ADC1C1 = gpioa.pa1.into_analog(&mut gpioa.crl);

        let adc_dma = adc1.with_scan_dma(AdcPins(adc_ch0, adc_ch1), dma1_ch1);

        defmt::info!("Configuring PWM virtual ground...");

        let tim2_ch3: pins::A2_PWM_VIRT_GND = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);

        let mut pwm = cx
            .device
            .TIM2
            .pwm_hz::<Tim2NoRemap, _, _>(tim2_ch3, &mut afio.mapr, config::clk::PCLK1, &clocks)
            .split();
        pwm.enable();
        pwm.set_duty(pwm.get_max_duty() / 2);

        defmt::info!("Configuring monotonic timer...");

        let mono = DwtSystick::new(
            &mut cx.core.DCB,
            cx.core.DWT,
            cx.core.SYST,
            clocks.sysclk().to_Hz(),
        );

        defmt::info!("Configuring debug indicator LED...");

        let led: pins::C13_DEBUG_LED = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, PinState::High);

        defmt::info!("Starting ADC DMA transfer...");

        let adc_dma_buf =
            singleton!(: [[u16; config::adc::BUF_LEN_PER_CHANNEL * 2]; 2] = [[0; config::adc::BUF_LEN_PER_CHANNEL * 2]; 2]).unwrap();
        let fft_buf =
            singleton!(: [i16; config::fft::BUF_LEN] = [0; config::fft::BUF_LEN]).unwrap();

        let adc_dma_transfer = adc_dma.circ_read(adc_dma_buf);

        defmt::info!("Finished init.");

        (
            Shared {},
            Local {
                adc_dma_transfer,
                fft_buf,
                debug_led: led,
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

    #[task(
        binds = DMA1_CHANNEL1,
        local = [
            adc_dma_transfer,
            fft_buf,
            debug_led,
        ],
        priority = 1
    )]
    fn adc_dma(cx: adc_dma::Context) {
        defmt::info!("Started processing ADC buffer...");

        let start = monotonics::now();
        cx.local.debug_led.set_low();

        let res = cx
            .local
            .adc_dma_transfer
            .peek(|half, _| adc::process_buffer(half, cx.local.fft_buf));

        cx.local.debug_led.set_high();
        let duration = monotonics::now() - start;

        match res {
            Ok(()) => defmt::info!(
                "Finished processing ADC buffer after {}us.",
                duration.to_micros()
            ),
            Err(_) => defmt::warn!(
                "ADC buffer processing did not complete in time (took {}us).",
                duration.to_micros()
            ),
        }
    }
}
