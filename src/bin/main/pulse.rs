use crate::config;
use crate::fft::analysis::Peak;
use crate::time::{Duration, Instant};
use heapless::Vec;

#[inline(never)]
pub fn schedule_pulses(
    peaks: &Vec<Peak, { config::fft::analysis::MAX_PEAKS }>,
    now: Instant,
    pulses_out: &mut Pulses,
) {
    assert!(pulses_out.pulses.capacity() == peaks.capacity());
    pulses_out.pulses.clear();
    pulses_out.pulses.extend(peaks.iter().map(|peak| {
        let period = peak.freq().into_duration();
        let phase_offset = peak.phase_offset();
        Pulse {
            period,
            next: now + phase_offset + period,
        }
    }));
}

pub struct Pulses {
    pulses: Vec<Pulse, { config::fft::analysis::MAX_PEAKS }>,
}

struct Pulse {
    period: Duration,
    next: Instant,
}

impl Pulses {
    pub const fn new() -> Self {
        Self { pulses: Vec::new() }
    }

    /// Consume a pulse scheduled for a specific instant, and reschedule the relevant frequencies.
    pub fn try_consume_pulse(&mut self, at: Instant) -> Result<(), ()> {
        let mut found_any_matching = false;
        for pulse in &mut self.pulses {
            if pulse.next == at {
                found_any_matching = true;
                pulse.next += pulse.period;
            }
        }
        if found_any_matching {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Get the timestamp of the next pulse, if any are scheduled.
    pub fn next_pulse(&self, after: Instant) -> Option<Instant> {
        self.pulses
            .iter()
            .min_by_key(|pulse| {
                let after_ticks = after.ticks();
                let pulse_ticks = pulse.next.ticks();
                // handle tick count wrapping, e.g.
                //
                // |-*---*----*-------------*---|
                //   ^   ^    ^      ^      ^
                //   2   3    4    after    1
                pulse_ticks.wrapping_sub(after_ticks)
            })
            .map(|pulse| pulse.next)
    }
}
