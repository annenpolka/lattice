use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::time::{Time, TimeError};

/// Mapping from an item's local time onto content (source) time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeMap {
    pub duration: Time,
    pub segments: Vec<TimeMapSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeMapSegment {
    /// Local time of the item, from the item start.
    pub local_start: Time,
    pub local_duration: Time,
    /// Content time at `local_start`.
    pub content_start: Time,
    /// Content delta per local delta. `0` is freeze, `1` is 1x, `-1` is reverse.
    pub rate: Time,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TimeMapError {
    #[error("time map is empty")]
    Empty,
    #[error("time {0} is outside the time map")]
    OutOfRange(Time),
    #[error("freeze duration must not be negative")]
    NegativeHold,
    #[error(transparent)]
    Time(#[from] TimeError),
}

impl TimeMap {
    pub fn identity(content_start: Time, duration: Time) -> Self {
        Self {
            duration,
            segments: vec![TimeMapSegment {
                local_start: Time::ZERO,
                local_duration: duration,
                content_start,
                rate: Time::ONE,
            }],
        }
    }

    pub fn content_at(&self, local: Time) -> Result<Time, TimeMapError> {
        if local < Time::ZERO || local > self.duration {
            return Err(TimeMapError::OutOfRange(local));
        }
        if local == self.duration {
            return self.content_at_end();
        }
        let segment = self
            .segments
            .iter()
            .find(|segment| {
                local >= segment.local_start && local < segment.local_start + segment.local_duration
            })
            .ok_or(TimeMapError::OutOfRange(local))?;
        let delta = local.checked_sub(segment.local_start)?;
        Ok(segment.content_start + delta * segment.rate)
    }

    fn content_at_end(&self) -> Result<Time, TimeMapError> {
        let segment = self.segments.last().ok_or(TimeMapError::Empty)?;
        let delta = segment.local_duration;
        Ok(segment.content_start + delta * segment.rate)
    }

    /// Insert a hold of `hold` local time at local time `at`. Content stays still.
    pub fn with_freeze(&self, at: Time, hold: Time) -> Result<Self, TimeMapError> {
        if hold < Time::ZERO {
            return Err(TimeMapError::NegativeHold);
        }
        if hold.is_zero() {
            return Ok(self.clone());
        }
        if at < Time::ZERO || at > self.duration {
            return Err(TimeMapError::OutOfRange(at));
        }
        let content = self.content_at(at)?;
        let mut segments = Vec::new();
        for segment in &self.segments {
            let start = segment.local_start;
            let end = start + segment.local_duration;
            if end <= at {
                segments.push(segment.clone());
                continue;
            }
            if start >= at {
                let mut shifted = segment.clone();
                shifted.local_start = start + hold;
                segments.push(shifted);
                continue;
            }
            let before = at.checked_sub(start)?;
            if !before.is_zero() {
                segments.push(TimeMapSegment {
                    local_start: start,
                    local_duration: before,
                    content_start: segment.content_start,
                    rate: segment.rate,
                });
            }
            segments.push(TimeMapSegment {
                local_start: at,
                local_duration: hold,
                content_start: content,
                rate: Time::ZERO,
            });
            let after = end.checked_sub(at)?;
            if !after.is_zero() {
                segments.push(TimeMapSegment {
                    local_start: at + hold,
                    local_duration: after,
                    content_start: content,
                    rate: segment.rate,
                });
            }
        }
        if at == self.duration {
            segments.push(TimeMapSegment {
                local_start: at,
                local_duration: hold,
                content_start: content,
                rate: Time::ZERO,
            });
        }
        Ok(Self {
            duration: self.duration.checked_add(hold)?,
            segments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(seconds: i64) -> Time {
        Time::seconds(seconds)
    }

    fn d(whole: i64, frac: i64, digits: u32) -> Time {
        Time::from_decimal_seconds(whole, frac, digits).unwrap()
    }

    #[test]
    fn freeze_in_the_middle() {
        let map = TimeMap::identity(s(10), s(10));
        let frozen = map.with_freeze(d(5, 2, 1), d(1, 5, 1)).unwrap();
        assert_eq!(frozen.duration, d(11, 5, 1));
        assert_eq!(frozen.content_at(Time::ZERO).unwrap(), s(10));
        assert_eq!(frozen.content_at(d(5, 2, 1)).unwrap(), d(15, 2, 1));
        assert_eq!(frozen.content_at(d(6, 7, 1)).unwrap(), d(15, 2, 1));
        assert_eq!(frozen.content_at(d(11, 5, 1)).unwrap(), s(20));
        assert_eq!(frozen.segments.len(), 3);
        assert_eq!(frozen.segments[1].rate, Time::ZERO);
    }

    #[test]
    fn identity_round_trip() {
        let map = TimeMap::identity(s(3), s(4));
        assert_eq!(map.content_at(s(0)).unwrap(), s(3));
        assert_eq!(map.content_at(s(4)).unwrap(), s(7));
    }
}
