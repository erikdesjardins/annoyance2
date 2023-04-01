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
            next: phase_offset,
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
            let mut dummy_pulse = Pulse {
                period: Duration::from_ticks(0),
                next: Instant::from_ticks(0),
            };
            let mut min_offset = Duration::from_ticks(u32::MAX);
            let mut next_pulse = &mut dummy_pulse;

            for pulse in &mut self.pulses {
                let after_ticks = after.ticks();
                let pulse_ticks = pulse.next.ticks();
                // handle tick count wrapping, e.g.
                //
                // |-*---*----*-------------*---|
                //   ^   ^    ^      ^      ^
                //   2   3    4    after    1
                let offset = Duration::from_ticks(pulse_ticks.wrapping_sub(after_ticks));
                if offset < min_offset {
                    min_offset = offset;
                    next_pulse = pulse;
                }
            }

            if min_offset < config::pulse::SCHEDULING_OFFSET {
                // too short interval--reschedule this pulse and retry
                next_pulse.next += next_pulse.period;
                continue;
            }

            if next_pulse.period == Duration::from_ticks(0) {
                break None;
            } else {
                break Some(next_pulse.next);
            };
        }
    }
}
