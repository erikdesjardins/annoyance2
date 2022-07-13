#![no_main]
#![no_std]
#![allow(
    clippy::assertions_on_constants,
    clippy::let_and_return,
    clippy::let_unit_value,
    clippy::needless_range_loop,
    clippy::redundant_pattern_matching,
    clippy::type_complexity
)]
#![warn(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::ptr_as_ptr
)]

use annoyance2 as _; // global logger + panicking-behavior + memory layout

mod adc;
mod collections;
mod config;
mod fft;
mod hal;
mod indicator;
mod math;
mod panic;
mod pulse;
mod time;

#[rtic::app(
    device = stm32f1xx_hal::pac,
    peripherals = true,
    dispatchers = [USART1, USART2, USART3]
)]
mod app {
    use crate::adc;
    use crate::config;
    use crate::fft;
    use crate::hal::pins;
    use crate::hal::tim::{OnePulse, OneshotTimer};
    use crate::indicator;
    use crate::math::ScaleBy;
    use crate::panic::OptionalExt;
    use crate::pulse;
    use crate::pulse::{Pulses, UnadjustedPulses};
    use crate::time::{Duration, Instant};
    use cortex_m::singleton;
    use dwt_systick_monotonic::DwtSystick;
    use heapless::Vec;
    use stm32f1xx_hal::adc::{Adc, AdcDma, Continuous};
    use stm32f1xx_hal::device::{ADC1, TIM1, TIM3, TIM4};
    use stm32f1xx_hal::dma::{dma1, CircBuffer, Event};
    use stm32f1xx_hal::gpio::PinState;
    use stm32f1xx_hal::prelude::*;
    use stm32f1xx_hal::timer::{
        Ch, Channel::*, PwmHz, Tim1NoRemap, Tim3NoRemap, Tim4NoRemap, Timer,
    };

    #[shared]
    struct Shared {
        pulses: &'static mut Pulses,
    }

    #[local]
    struct Local {
        adc_dma_transfer: CircBuffer<
            [u16; config::adc::BUF_LEN_RAW],
            AdcDma<ADC1, pins::A0_ADC1C0, Continuous, dma1::C1>,
        >,
        fft_buf: &'static mut [i16; config::fft::BUF_LEN_REAL],
        next_pulses: &'static mut UnadjustedPulses,
        amplitude_timer: PwmHz<
            TIM3,
            Tim3NoRemap,
            (Ch<0>, Ch<1>, Ch<2>, Ch<3>),
            (
                pins::A6_TIM3C1,
                pins::A7_TIM3C2,
                pins::B0_TIM3C3,
                pins::B1_TIM3C4,
            ),
        >,
        threshold_timer: PwmHz<
            TIM4,
            Tim4NoRemap,
            (Ch<0>, Ch<1>, Ch<2>, Ch<3>),
            (
                pins::B6_TIM4C1,
                pins::B7_TIM4C2,
                pins::B8_TIM4C3,
                pins::B9_TIM4C4,
            ),
        >,
        pulse_timer:
            OnePulse<TIM1, Tim1NoRemap, Ch<0>, pins::A8_TIM1C1_PULSE, { config::clk::TIM1CLK_HZ }>,
        debug_led: pins::C13_DEBUG_LED,
    }

    #[init]
    fn init(mut cx: init::Context) -> (Shared, Local, init::Monotonics) {
        defmt::info!("Dumping config...");

        config::dump_to_log();

        defmt::info!("Starting init...");

        let mut afio = cx.device.AFIO.constrain();
        let dma1 = cx.device.DMA1.split();
        let mut flash = cx.device.FLASH.constrain();
        let mut gpioa = cx.device.GPIOA.split();
        let mut gpiob = cx.device.GPIOB.split();
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

        let adc_dma = adc1.with_dma(adc_ch0, dma1_ch1);

        defmt::info!("Configuring amplitude indicator timer...");

        let tim3_ch4: pins::B1_TIM3C4 = gpiob.pb1.into_alternate_push_pull(&mut gpiob.crl);
        let tim3_ch3: pins::B0_TIM3C3 = gpiob.pb0.into_alternate_push_pull(&mut gpiob.crl);
        let tim3_ch2: pins::A7_TIM3C2 = gpioa.pa7.into_alternate_push_pull(&mut gpioa.crl);
        let tim3_ch1: pins::A6_TIM3C1 = gpioa.pa6.into_alternate_push_pull(&mut gpioa.crl);

        let mut amplitude_timer = Timer::new(cx.device.TIM3, &clocks).pwm_hz(
            (tim3_ch1, tim3_ch2, tim3_ch3, tim3_ch4),
            &mut afio.mapr,
            config::indicator::PWM_FREQ,
        );
        for ch in [C1, C2, C3, C4] {
            amplitude_timer.set_duty(ch, 0);
            amplitude_timer.enable(ch);
        }

        defmt::info!("Configuring threshold indicator timer...");

        let tim4_ch1: pins::B6_TIM4C1 = gpiob.pb6.into_alternate_push_pull(&mut gpiob.crl);
        let tim4_ch2: pins::B7_TIM4C2 = gpiob.pb7.into_alternate_push_pull(&mut gpiob.crl);
        let tim4_ch3: pins::B8_TIM4C3 = gpiob.pb8.into_alternate_push_pull(&mut gpiob.crh);
        let tim4_ch4: pins::B9_TIM4C4 = gpiob.pb9.into_alternate_push_pull(&mut gpiob.crh);

        let mut threshold_timer = Timer::new(cx.device.TIM4, &clocks).pwm_hz(
            (tim4_ch1, tim4_ch2, tim4_ch3, tim4_ch4),
            &mut afio.mapr,
            config::indicator::PWM_FREQ,
        );
        for ch in [C1, C2, C3, C4] {
            threshold_timer.set_duty(ch, 0);
            threshold_timer.enable(ch);
        }

        defmt::info!("Configuring pulse output timer...");

        let tim1_ch1: pins::A8_TIM1C1_PULSE = gpioa.pa8.into_alternate_push_pull(&mut gpioa.crh);

        let pulse_timer = OneshotTimer::new(cx.device.TIM1, &clocks).one_pulse_mode(
            tim1_ch1,
            &mut afio.mapr,
            config::pulse::DURATION,
        );

        defmt::info!("Configuring monotonic timer...");

        let mono = DwtMono::new(
            &mut cx.core.DCB,
            cx.core.DWT,
            cx.core.SYST,
            clocks.sysclk().to_Hz(),
        );

        defmt::info!("Configuring debug indicator LED...");

        let led: pins::C13_DEBUG_LED = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, PinState::High);

        defmt::info!("Preparing buffers...");

        let adc_dma_buf =
            singleton!(: [[u16; config::adc::BUF_LEN_RAW]; 2] = [[0; config::adc::BUF_LEN_RAW]; 2])
                .unwrap();

        let fft_buf =
            singleton!(: [i16; config::fft::BUF_LEN_REAL] = [0; config::fft::BUF_LEN_REAL])
                .unwrap();

        let pulses = singleton!(: Pulses = Pulses::new()).unwrap();

        let next_pulses = singleton!(: UnadjustedPulses = UnadjustedPulses::new()).unwrap();

        defmt::info!("Starting ADC DMA transfer...");

        let adc_dma_transfer = adc_dma.circ_read(adc_dma_buf);

        defmt::info!("Finished init.");

        (
            Shared { pulses },
            Local {
                adc_dma_transfer,
                fft_buf,
                next_pulses,
                amplitude_timer,
                threshold_timer,
                pulse_timer,
                debug_led: led,
            },
            init::Monotonics(mono),
        )
    }

    // Task priorities
    //
    // Prio | Task         | Description
    //   16 | fire_pulse   | outputs pulses (triggered by timer interrupt)
    //   15 | DwtMono      | monotonic timer interrupt
    //   14 | swap_buffers | schedules pulse timing and processes ADC buffers
    //    0 | idle         | idle task

    /// This provides a monotonic timer used to trigger scheduled tasks.
    #[monotonic(
        binds = SysTick,
        priority = 15,
        default = true
    )]
    type DwtMono = DwtSystick<{ config::clk::SYSCLK_HZ }>;

    /// This task fires pulses.
    ///
    /// This task has the highest priority since any jitter will add phase noise to the output.
    #[task(
        shared = [
            pulses,
        ],
        local = [
            pulse_timer,
        ],
        priority = 16,
        capacity = 2 /* self + swap_buffers */,
    )]
    fn fire_pulse(mut cx: fire_pulse::Context, now: Instant) {
        cx.shared.pulses.lock(|pulses| {
            // consume this pulse (rescheduling this frequency)
            if let Err(()) = pulses.try_consume_pulse(now) {
                // if this pulse isn't present in the pulse train, our buffer was swapped,
                // and we've already been rescheduled
                defmt::trace!("pulse skipped due to in-flight buffer swap");
                return;
            }

            // fire timer
            cx.local.pulse_timer.reset_and_fire();
            if config::debug::LOG_ALL_PULSES {
                defmt::println!(
                    "Firing pulse at {}us",
                    Duration::from_ticks(now.ticks()).to_micros()
                );
            }

            // reschedule ourselves for the next pulse
            if let Some(next_pulse) = pulses.next_pulse(now) {
                if let Err(_) = fire_pulse::spawn_at(next_pulse, next_pulse) {
                    defmt::warn!("fire_pulse schedule overrun");
                }
            }
        });
    }

    /// This task schedules pulse timings, from the previous buffer,
    /// to be emitted while processing the current buffer.
    ///
    /// Ideally, we would like this task to have no jitter,
    /// so that timings are computed based on a consistent interval.
    ///
    /// However, this isn't perfectly feasible, since `fire_pulse` needs to be higher priority.
    ///
    /// Note that even having the highest priority would not guarantee lack of jitter:
    /// resource locks can introduce jitter when a lower-priority task takes the lock.
    #[task(
        binds = DMA1_CHANNEL1,
        shared = [
            pulses,
        ],
        local = [
            adc_dma_transfer,
            fft_buf,
            next_pulses,
            amplitude_timer,
            threshold_timer,
            debug_led,
        ],
        priority = 14,
    )]
    fn swap_buffers(mut cx: swap_buffers::Context) {
        cx.local.debug_led.set_low();

        // getting the timestamp must happen a consistent delay after the start of the task,
        // so timing is consistent
        let start = monotonics::now();
        // ...after this point, even if there is variable latency / jitter,
        //    everything will be scheduled off of that timestamp.

        // Note that we still need to start scheduling pulses quickly, however,
        // because delays could result in pulses being scheduled in the past.

        // Phase 1: swap in new pulse train and reschedule

        // Step 1: adjust pulses and swap
        let next_pulse = cx.shared.pulses.lock(|pulses| {
            pulses.replace_with_adjusted(cx.local.next_pulses, start);
            pulses.next_pulse(start)
        });

        // Step 2: schedule task for next pulse
        if let Some(next_pulse) = next_pulse {
            if let Err(_) = fire_pulse::spawn_at(next_pulse, next_pulse) {
                defmt::warn!("fire_pulse schedule overrun (from swap_buffers2)");
            }
        }

        let mid = monotonics::now();
        defmt::debug!(
            "Finished scheduling new pulses after {}us",
            (mid - start).to_micros()
        );

        // Phase 2: process current ADC buffer to prepare for the next swap

        let res = cx.local.adc_dma_transfer.peek(|samples, _| {
            let scratch = cx.local.fft_buf;

            let (values, padding) = scratch.split_at_mut(config::adc::BUF_LEN_PROCESSED);
            let values: &mut [_; config::adc::BUF_LEN_PROCESSED] =
                values.try_into().unwrap_infallible();

            // Step 0: compute and display amplitude from raw samples
            let amplitude_factors = indicator::amplitude_scaling_factors(samples);
            for (factor, ch) in amplitude_factors.into_iter().zip([C4, C3, C2, C1]) {
                let duty = cx.local.amplitude_timer.get_max_duty().scale_by(factor);
                cx.local.amplitude_timer.set_duty(ch, duty);
            }

            // Step 1: populate values and padding in FFT scratch buffer
            adc::process_raw_samples(samples, values);
            padding.fill(0);

            // Step 2: apply window function to data
            fft::window::apply_to(values);

            // Step 3: run fft
            let bins = fft::run(scratch);

            // Step 4: find peaks in spectrum
            let mut peaks = Vec::new();
            fft::analysis::find_peaks(bins, &mut peaks);

            // Step 5: compute pulses based on peaks
            pulse::schedule_pulses(&peaks, cx.local.next_pulses);

            // Step 6: compute and display "above threshold" from peaks
            let threshold_factors = indicator::threshold_scaling_factors(&peaks);
            for (factor, ch) in threshold_factors.into_iter().zip([C1, C2, C3, C4]) {
                let duty = cx.local.threshold_timer.get_max_duty().scale_by(factor);
                cx.local.threshold_timer.set_duty(ch, duty);
            }
        });

        let duration = monotonics::now() - mid;
        match res {
            Ok(()) => defmt::debug!(
                "Finished processing ADC buffer after {}us.",
                duration.to_micros()
            ),
            Err(_) => defmt::warn!(
                "ADC buffer processing did not complete in time (took {}us).",
                duration.to_micros()
            ),
        }

        cx.local.debug_led.set_high();
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {
            // Note that using `wfi` here breaks debugging,
            // so if desired we should only do that in release mode.
            continue;
        }
    }
}
