use crate::collections::ReplaceWithMapped;
use crate::config;
use crate::fft::analysis::Peak;
use crate::time::{Duration, Instant};
use heapless::Vec;

/// A pulse, based on a timestamp that may be a duration in the future or a realtime timestamp.
struct Pulse<Next> {
    period: Duration,
    next: Next,
}

#[inline(never)]
pub fn schedule_pulses(
    peaks: &Vec<Peak, { config::fft::analysis::MAX_PEAKS }>,
    pulses_out: &mut UnadjustedPulses,
) {
    pulses_out.pulses.replace_with_mapped(peaks, |peak| {
        let period = peak.period();
        let phase_offset = peak.phase_offset();
        Pulse {
            period,
            next: phase_offset + period,
        }
    });
}

/// Holds pulses which contain their phase offset + period,
/// but need to be adjusted by adding a start timestamp.
pub struct UnadjustedPulses {
    pulses: Vec<Pulse<Duration>, { config::fft::analysis::MAX_PEAKS }>,
}

impl UnadjustedPulses {
    pub const fn new() -> Self {
        Self { pulses: Vec::new() }
    }
}

/// Holds pulses which are scheduled based on a realtime timestamp.
pub struct Pulses {
    pulses: Vec<Pulse<Instant>, { config::fft::analysis::MAX_PEAKS }>,
}

impl Pulses {
    pub const fn new() -> Self {
        Self { pulses: Vec::new() }
    }

    pub fn replace_with_adjusted(&mut self, unadjusted: &UnadjustedPulses, at: Instant) {
        self.pulses
            .replace_with_mapped(&unadjusted.pulses, |pulse| Pulse {
                period: pulse.period,
                next: at + pulse.next,
            })
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
    pub fn next_pulse(&mut self, after: Instant) -> Option<Instant> {
        loop {
            let (offset, next_pulse) = self
                .pulses
                .iter()
                .map(|pulse| {
                    let after_ticks = after.ticks();
                    let pulse_ticks = pulse.next.ticks();
                    // handle tick count wrapping, e.g.
                    //
                    // |-*---*----*-------------*---|
                    //   ^   ^    ^      ^      ^
                    //   2   3    4    after    1
                    let offset = Duration::from_ticks(pulse_ticks.wrapping_sub(after_ticks));
                    (offset, pulse.next)
                })
                .min_by_key(|(offset, _)| *offset)?;

            if offset < config::pulse::SCHEDULING_OFFSET {
                // too short interval--discard this pulse and retry
                self.try_consume_pulse(next_pulse)
                    .unwrap_or_else(|_| panic!("can't find pulse that exists (impossible)"));
                continue;
            }

            break Some(next_pulse);
        }
    }
}
