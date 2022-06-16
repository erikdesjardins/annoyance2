use core::marker::PhantomData;
use fugit::Duration;
use stm32f1xx_hal::afio::MAPR;
use stm32f1xx_hal::device::{RCC, TIM1};
use stm32f1xx_hal::rcc::Clocks;
use stm32f1xx_hal::timer::pwm::Pins;
use stm32f1xx_hal::timer::{Instance, Ocm, Remap};

pub struct OneshotTimer<TIM: Instance, const FREQ: u32> {
    tim: TIM,
}

// modified from
// https://github.com/stm32-rs/stm32f1xx-hal/blob/f9b24f4d9bac7fc3c93764bd295125800944f53b/src/timer.rs#L713-L735
impl<TIM: Instance, const FREQ: u32> OneshotTimer<TIM, FREQ> {
    pub fn new(tim: TIM, clocks: &Clocks) -> Self {
        unsafe {
            //NOTE(unsafe) this reference will only be used for atomic writes with no side effects
            let rcc = &(*RCC::ptr());
            // Enable and reset the timer peripheral
            TIM::enable(rcc);
            TIM::reset(rcc);
        }

        let mut t = Self { tim };
        t.configure(clocks);
        t
    }

    /// Calculate prescaler depending on `Clocks` state
    fn configure(&mut self, clocks: &Clocks) {
        let clk = TIM::timer_clock(clocks);
        assert!(clk.raw() % FREQ == 0);
        let psc = clk.raw() / FREQ;
        self.tim.set_prescaler(u16::try_from(psc - 1).unwrap());
    }
}

pub struct OnePulse<TIM, REMAP, P, PINS, const FREQ: u32>
where
    TIM: Instance,
    REMAP: Remap<Periph = TIM>,
    PINS: Pins<REMAP, P>,
{
    timer: OneshotTimer<TIM, FREQ>,
    _pins: PhantomData<(REMAP, P, PINS)>,
}

// modified from
// https://github.com/stm32-rs/stm32f1xx-hal/blob/f9b24f4d9bac7fc3c93764bd295125800944f53b/src/timer/pwm.rs#L437-L484
impl<const FREQ: u32> OneshotTimer<TIM1, FREQ> {
    pub fn one_pulse_mode<REMAP, P, PINS>(
        self,
        _pins: PINS,
        mapr: &mut MAPR,
        pulse_time: Duration<u32, 1, FREQ>,
    ) -> OnePulse<TIM1, REMAP, P, PINS, FREQ>
    where
        REMAP: Remap<Periph = TIM1>,
        PINS: Pins<REMAP, P>,
    {
        REMAP::remap(mapr);

        // 0 -> 1 at CCR, 1 -> 0 at ARR
        let mode = Ocm::PwmMode2;

        if PINS::C1 {
            self.tim.ccmr1_output().modify(|_, w| {
                w
                    // enable preload on CCR
                    .oc1pe()
                    .set_bit()
                    // set output control mode
                    .oc1m()
                    .bits(mode as _)
                    // enable fast enable (do not wait for CCR comparison)
                    // this is referred to as "Particular case: OCx fast enable" in
                    // https://www.st.com/resource/en/reference_manual/rm0008-stm32f101xx-stm32f102xx-stm32f103xx-stm32f105xx-and-stm32f107xx-advanced-armbased-32bit-mcus-stmicroelectronics.pdf
                    .oc1fe()
                    .set_bit()
            });
            // CCR must be > 0, but is otherwise ignored here due to fast enable
            self.tim.ccr1.write(|w| w.ccr().bits(1));
            // Enable the capture/compare channel
            self.tim.ccer.modify(|_, w| w.cc1e().set_bit());
        }
        if PINS::C2 {
            self.tim
                .ccmr1_output()
                .modify(|_, w| w.oc2pe().set_bit().oc2m().bits(mode as _).oc2fe().set_bit());
            self.tim.ccr2.write(|w| w.ccr().bits(1));
            self.tim.ccer.modify(|_, w| w.cc2e().set_bit());
        }
        if PINS::C3 {
            self.tim
                .ccmr2_output()
                .modify(|_, w| w.oc3pe().set_bit().oc3m().bits(mode as _).oc3fe().set_bit());
            self.tim.ccr3.write(|w| w.ccr().bits(1));
            self.tim.ccer.modify(|_, w| w.cc3e().set_bit());
        }
        if PINS::C4 {
            self.tim
                .ccmr2_output()
                .modify(|_, w| w.oc4pe().set_bit().oc4m().bits(mode as _).oc4fe().set_bit());
            self.tim.ccr4.write(|w| w.ccr().bits(1));
            self.tim.ccer.modify(|_, w| w.cc4e().set_bit());
        }

        // Enable preload for ARR
        self.tim.cr1.modify(|_, w| w.arpe().bit(true));

        // time is ARR - CCR + 1, so subtract 1 tick
        // (note that CCR is effectively 0 here due to fast enable)
        self.tim.arr.write(|w| {
            w.arr().bits({
                let ticks = pulse_time.ticks() - 1;
                assert!(ticks > 0);
                ticks.try_into().unwrap()
            })
        });

        // Trigger update event to load the registers
        // (also sets the URS bit to prevent an interrupt from being triggered by the UG bit)
        self.tim.cr1.modify(|_, w| w.urs().set_bit());
        self.tim.egr.write(|w| w.ug().set_bit());
        self.tim.cr1.modify(|_, w| w.urs().clear_bit());

        // Set automatic output enable (for TIM1 only)
        self.tim.bdtr.modify(|_, w| w.aoe().set_bit());

        OnePulse {
            timer: self,
            _pins: PhantomData,
        }
    }
}

impl<REMAP, P, PINS, const FREQ: u32> OnePulse<TIM1, REMAP, P, PINS, FREQ>
where
    REMAP: Remap<Periph = TIM1>,
    PINS: Pins<REMAP, P>,
{
    pub fn reset_and_fire(&mut self) {
        // enable one pulse mode and start the timer
        self.timer
            .tim
            .cr1
            .write(|w| w.opm().set_bit().cen().set_bit());
    }
}
