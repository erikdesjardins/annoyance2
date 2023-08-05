use crate::collections::ReplaceWithMapped;
use crate::config;
use crate::fft::analysis::Peak;
use crate::math::ScaleBy;
use crate::time::{Duration, Instant};
use heapless::Vec;

/// A pulse, based on a timestamp that may be a duration in the future or a realtime timestamp.
#[derive(Copy, Clone)]
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
    holdoff_until: Instant,
}

impl Pulses {
    pub const fn new(at: Instant) -> Self {
        Self {
            pulses: Vec::new(),
            holdoff_until: at,
        }
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
        if self.pulses.is_empty() {
            return None;
        }

        // Compute min offset to meet holdoff time
        let holdoff = {
            let after_ticks = after.ticks();
            let holdoff_ticks = self.holdoff_until.ticks();
            // Handle tick count wrapping, e.g.
            //
            // |-*------------------*-------|
            //   ^                  ^   ^
            //  h2                 h1 after
            //
            // Holdoff 1 just expired, so we should consider it zero holdoff time.
            // Holdoff 2 is still in the future, so it should be some positive holdoff time.
            // We distinguish these by assuming any tick count difference less than half the range is in the future,
            // and anything more than half the range is in the past.
            let raw_offset = holdoff_ticks.wrapping_sub(after_ticks);
            if raw_offset < u32::MAX / 2 {
                Duration::from_ticks(raw_offset)
            } else {
                Duration::from_ticks(0)
            }
        };
        // Ensure holdoff is at least the minimum
        let holdoff = holdoff.max(config::pulse::MIN_HOLDOFF);

        let mut min_offset = Duration::from_ticks(u32::MAX);
        let mut next_pulse = &mut Pulse {
            period: Duration::from_ticks(0),
            next: Instant::from_ticks(0),
        };

        for pulse in &mut self.pulses {
            let after_ticks = after.ticks();
            let pulse_ticks = pulse.next.ticks();
            // Handle tick count wrapping, e.g.
            //
            // |-*---*----*-------------*---|
            //   ^   ^    ^      ^      ^
            //   2   3    4    after    1
            let mut offset = Duration::from_ticks(pulse_ticks.wrapping_sub(after_ticks));

            while offset < holdoff {
                // reschedule this pulse until it happens after the holdoff
                pulse.next += pulse.period;
                offset += pulse.period;
            }

            if offset < min_offset {
                min_offset = offset;
                next_pulse = pulse;
            }
        }

        // Update holdoff time for latest pulse
        self.holdoff_until =
            next_pulse.next + next_pulse.period.scale_by(config::pulse::HOLDOFF_RATIO);
        Some(next_pulse.next)
    }
}
