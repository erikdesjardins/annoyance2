#![no_main]
#![no_std]
#![allow(
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
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
mod config;
mod fft;
mod hal;
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
    use crate::fft::analysis::Peak;
    use crate::hal::pins;
    use crate::hal::tim::{OnePulse, OneshotTimer};
    use crate::hal::AdcPins;
    use crate::panic::OptionalExt;
    use crate::pulse;
    use crate::pulse::Pulses;
    use crate::time::Instant;
    use core::mem;
    use cortex_m::singleton;
    use dwt_systick_monotonic::DwtSystick;
    use heapless::Vec;
    use stm32f1xx_hal::adc::{Adc, AdcDma, Scan};
    use stm32f1xx_hal::device::{ADC1, TIM1};
    use stm32f1xx_hal::dma::{dma1, CircBuffer, Event};
    use stm32f1xx_hal::gpio::PinState;
    use stm32f1xx_hal::prelude::*;
    use stm32f1xx_hal::timer::{Ch, Tim1NoRemap, Tim2NoRemap};

    #[shared]
    struct Shared {
        peaks: &'static mut Vec<Peak, { config::fft::analysis::MAX_PEAKS }>,
        pulses: &'static mut Pulses,
    }

    #[local]
    struct Local {
        adc_dma_transfer:
            CircBuffer<[u16; config::adc::BUF_LEN_RAW], AdcDma<ADC1, AdcPins, Scan, dma1::C1>>,
        fft_buf: &'static mut [i16; config::fft::BUF_LEN_REAL],
        next_pulses: &'static mut Pulses,
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
        let adc_ch1: pins::A4_ADC1C4 = gpioa.pa4.into_analog(&mut gpioa.crl);

        let adc_dma = adc1.with_scan_dma(AdcPins(adc_ch0, adc_ch1), dma1_ch1);

        defmt::info!("Configuring PWM virtual ground...");

        let tim2_ch3: pins::A2_PWM_VIRT_GND = gpioa.pa2.into_alternate_push_pull(&mut gpioa.crl);

        let mut pwm = cx
            .device
            .TIM2
            .pwm_hz::<Tim2NoRemap, _, _>(tim2_ch3, &mut afio.mapr, config::clk::PCLK1, &clocks)
            .split();
        pwm.set_duty(pwm.get_max_duty() / 2);
        pwm.enable();

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

        let peaks =
            singleton!(: Vec<Peak, {config::fft::analysis::MAX_PEAKS}> = Vec::new()).unwrap();

        const PULSES: Pulses = Pulses::new();
        let [pulses, next_pulses] = singleton!(: [Pulses; 2] = [PULSES; 2]).unwrap();

        defmt::info!("Starting ADC DMA transfer...");

        let adc_dma_transfer = adc_dma.circ_read(adc_dma_buf);

        defmt::info!("Finished init.");

        (
            Shared { peaks, pulses },
            Local {
                adc_dma_transfer,
                fft_buf,
                next_pulses,
                pulse_timer,
                debug_led: led,
            },
            init::Monotonics(mono),
        )
    }

    // Task priorities
    //
    // Prio | Task           | Description
    //   15 | swap_buffers   | starts scheduling pulse timing (must have no jitter)
    //   14 | fire_pulse     | outputs pulses (triggered by timer interrupt)
    //   13 | DwtMono        | monotonic timer interrupt
    //   12 | swap_buffers2  | finishes scheduling pulse timing (triggered by swap_buffers)
    //    1 | process_buffer | processes ADC buffers (long batch task)
    //    0 | idle           | idle task

    // Task scheduling
    //
    // x = blocked from running due to scheduling prio
    // o = running
    //
    // Time  | idle | process_buffer | swap_buffers2 | DwtMono | fire_pulse | swap_buffers
    // 0 cyc    x            x               x            x          x        oooooooooooo  <- DMA completes half transfer, triggers swap_buffers
    //          x            x         ooooooooooooo <-----------------------/              <- swap_buffers triggers swap_buffers2
    //          x            x         ooooooooooooo
    //          x            x               x         ooooooo                              <- pulses may fire during swap_buffers2
    //          x            x               x            x  \-> oooooooooo
    //          x            x         ooooooooooooo
    //          x            x         ooooooooooooo                                        <- swap_buffers2 completes, schedules fire_pulse
    //          x     oooooooooooooo <-/                                                    <- swap_buffers2 triggers process_buffer
    //          x     oooooooooooooo
    //          x     oooooooooooooo
    //          x            x               x         ooooooo                              <- pulses will fire during process_buffer
    //          x            x               x            x  \-> oooooooooo
    //          x     oooooooooooooo
    //          x     oooooooooooooo
    //          x     oooooooooooooo                                                        <- process_buffer completes
    //         oooo
    //         oooo
    //          x            x               x         ooooooo                              <- pulses will fire after processing
    //          x            x               x            x  \-> oooooooooo
    //         oooo
    //         oooo
    // 1 cyc

    /// This provides a monotonic timer used trigger scheduled tasks.
    #[monotonic(
        binds = SysTick,
        priority = 13,
        default = true
    )]
    type DwtMono = DwtSystick<{ config::clk::SYSCLK_HZ }>;

    /// This task schedules pulse timings, from the previous buffer,
    /// to be emitted while processing the current buffer.
    ///
    /// It is critical that this task has the highest priority and no jitter,
    /// so that timings are computed based on a consistent interval.
    ///
    /// Note that having the highest priority alone does not guarantee lack of jitter:
    /// if this task shares a resource with a lower priority task,
    /// that task will lock the resource while it's being accessed.
    ///
    /// It is also critical that this task is very short,
    /// since it will block the execution of other tasks, in particular `fire_pulse`.
    #[task(
        binds = DMA1_CHANNEL1,
        priority = 15,
    )]
    fn swap_buffers(_cx: swap_buffers::Context) {
        // getting the timestamp must happen first, so timing is consistent
        let now = monotonics::now();
        // ...after this point, even if there is variable latency / jitter,
        //    everything will be scheduled off of a consistent timer,
        //    so at most the first (few) pulses will "pile up" if we miss our deadline,
        //    but the entire buffer's timing won't be misaligned.

        // while we could continue in this task, switch to a lower priority task:
        // - this ensures that we don't block the continued firing of pulses while scheduling;
        // - more importantly, that allows this task to have no shared state, and hence no locks
        if let Err(_) = swap_buffers2::spawn(now) {
            defmt::warn!("swap_buffers2 overrun (should never happen)");
        }
    }

    /// This task has lower priority than `fire_pulses`, so it does not block pulses while it runs.
    /// However, it still needs to complete quickly, so as to not drop initial pulses from the new buffer.
    #[task(
        shared = [
            peaks,
            pulses,
        ],
        local = [
            next_pulses,
        ],
        priority = 12,
    )]
    fn swap_buffers2(mut cx: swap_buffers2::Context, now: Instant) {
        // schedule pulse train
        // (this blocks process_buffer)
        cx.shared.peaks.lock(|peaks| {
            pulse::schedule_pulses(peaks, now, cx.local.next_pulses);
        });

        // swap in new pulse train and reschedule
        let next_pulse = cx.local.next_pulses.next_pulse(now);
        // (this blocks fire_pulse)
        cx.shared.pulses.lock(|pulses: &mut &mut _| {
            mem::swap(pulses, &mut cx.local.next_pulses);
        });
        if let Some(next_pulse) = next_pulse {
            if let Err(_) = fire_pulse::spawn_at(next_pulse, next_pulse) {
                defmt::warn!("fire_pulse schedule overrun (from swap_buffers2)");
            }
        }

        // start processing the ADC buffer that triggered the interrupt for `swap_buffers`
        if let Err(()) = process_buffer::spawn() {
            defmt::warn!("ADC buffer overrun");
        }
    }

    /// This task fires pulses.
    ///
    /// Like `swap_buffers`, this task has very high priority
    /// as any jitter will add phase noise to the output.
    ///
    /// However, it is still lower priority than `swap_buffers`, because a delay in `swap_buffers`
    /// will affect every pulse, but a delay in `fire_pulse` will only affect one pulse.
    #[task(
        shared = [
            pulses,
        ],
        local = [
            pulse_timer,
        ],
        priority = 14,
        capacity = 2 /* self + swap_buffers2 */,
    )]
    fn fire_pulse(mut cx: fire_pulse::Context, now: Instant) {
        cx.shared.pulses.lock(|pulses| {
            // consume this pulse (rescheduling this frequency)
            if let Err(()) = pulses.try_consume_pulse(now) {
                // if this pulse isn't present in the pulse train, our buffer was swapped,
                // and `swap_buffers2` already rescheduled us
                defmt::debug!("pulse skipped due to in-flight buffer swap");
                return;
            }

            // fire timer
            cx.local.pulse_timer.reset_and_fire();

            // reschedule ourselves for the next pulse
            if let Some(next_pulse) = pulses.next_pulse(now) {
                if let Err(_) = fire_pulse::spawn_at(next_pulse, next_pulse) {
                    defmt::warn!("fire_pulse schedule overrun");
                }
            }
        });
    }

    #[task(
        shared = [
            peaks,
        ],
        local = [
            adc_dma_transfer,
            fft_buf,
            debug_led,
        ],
        priority = 1,
    )]
    fn process_buffer(mut cx: process_buffer::Context) {
        defmt::debug!("Started processing ADC buffer...");

        let start = monotonics::now();
        cx.local.debug_led.set_low();

        let res = cx.local.adc_dma_transfer.peek(|samples, _| {
            let scratch = cx.local.fft_buf;

            let (values, padding) = scratch.split_at_mut(config::adc::BUF_LEN_PROCESSED);
            let values: &mut [_; config::adc::BUF_LEN_PROCESSED] =
                values.try_into().unwrap_infallible();

            // populate values and padding in FFT scratch buffer
            adc::process_raw_samples(samples, values);
            padding.fill(0);

            // apply window function to data
            fft::window::apply_to(values);

            // run fft
            let bins = fft::run(scratch);

            // find peaks in spectrum
            cx.shared.peaks.lock(|peaks| {
                fft::analysis::find_peaks(bins, peaks);
            });
        });

        cx.local.debug_led.set_high();
        let duration = monotonics::now() - start;

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
