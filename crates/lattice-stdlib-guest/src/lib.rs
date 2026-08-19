//! Stdlib WIT guest. Emits Core-compatible semantics, never FFmpeg or GPUI.

wit_bindgen::generate!({
    world: "stdlib",
    path: "../../wit/lattice/stdlib.wit",
});

use crate::exports::lattice::stdlib::lowering::{
    Guest, PlacementFragment, RationalTime, Span, TimeMap, TimeMapSegment,
};
use lattice_core::{Time, TimeMap as CoreTimeMap, TimeMapSegment as CoreSegment};

struct Stdlib;

export!(Stdlib);

impl Guest for Stdlib {
    fn freeze(map: TimeMap, at: RationalTime, hold: RationalTime) -> Result<TimeMap, String> {
        let map = from_wit_map(map)?;
        let at = from_wit_time(at)?;
        let hold = from_wit_time(hold)?;
        let updated = map.with_freeze(at, hold).map_err(|err| err.to_string())?;
        Ok(to_wit_map(&updated))
    }

    fn title(
        text: String,
        at: RationalTime,
        hold: RationalTime,
        opacity: Option<u8>,
        span: Span,
    ) -> Result<PlacementFragment, String> {
        if text.is_empty() {
            return Err("`title` needs a string".into());
        }
        let at = from_wit_time(at)?;
        let hold = from_wit_time(hold)?;
        if hold < Time::ZERO {
            return Err("title duration must not be negative".into());
        }
        Ok(PlacementFragment {
            start: to_wit_time(at),
            duration: to_wit_time(hold),
            text,
            opacity,
            span,
        })
    }
}

fn from_wit_time(time: RationalTime) -> Result<Time, String> {
    Time::new(time.num, time.den).map_err(|err| err.to_string())
}

fn to_wit_time(time: Time) -> RationalTime {
    RationalTime {
        num: time.num(),
        den: time.den(),
    }
}

fn from_wit_map(map: TimeMap) -> Result<CoreTimeMap, String> {
    let duration = from_wit_time(map.duration)?;
    let mut segments = Vec::new();
    for segment in map.segments {
        segments.push(CoreSegment {
            local_start: from_wit_time(segment.local_start)?,
            local_duration: from_wit_time(segment.local_duration)?,
            content_start: from_wit_time(segment.content_start)?,
            rate: from_wit_time(segment.rate)?,
        });
    }
    Ok(CoreTimeMap {
        duration,
        segments,
    })
}

fn to_wit_map(map: &CoreTimeMap) -> TimeMap {
    TimeMap {
        duration: to_wit_time(map.duration),
        segments: map
            .segments
            .iter()
            .map(|segment| TimeMapSegment {
                local_start: to_wit_time(segment.local_start),
                local_duration: to_wit_time(segment.local_duration),
                content_start: to_wit_time(segment.content_start),
                rate: to_wit_time(segment.rate),
            })
            .collect(),
    }
}
