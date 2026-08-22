//! Backend-neutral animated properties. `evaluate(t)` lowers these to a snapshot;
//! `FrameRenderer` sees concrete values, not curves.

use serde::{Deserialize, Serialize};

use crate::time::Time;

/// Linear interpolation parameter: `num/den` in `[0, 1]`.
pub trait Interpolate: Clone {
    fn interpolate(a: &Self, b: &Self, num: i64, den: i64) -> Self;
}

impl Interpolate for u8 {
    fn interpolate(a: &Self, b: &Self, num: i64, den: i64) -> Self {
        let value = lerp_i64(i64::from(*a), i64::from(*b), num, den).clamp(0, 255);
        u8::try_from(value).unwrap_or(0)
    }
}

impl Interpolate for i32 {
    fn interpolate(a: &Self, b: &Self, num: i64, den: i64) -> Self {
        let value = lerp_i64(i64::from(*a), i64::from(*b), num, den);
        i32::try_from(value).unwrap_or(0)
    }
}

impl Interpolate for u32 {
    fn interpolate(a: &Self, b: &Self, num: i64, den: i64) -> Self {
        let value = lerp_i64(i64::from(*a), i64::from(*b), num, den).max(0);
        u32::try_from(value).unwrap_or(0)
    }
}

/// v0 easing: linear only. No shader/easing DSL.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Easing {
    #[default]
    Linear,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Keyframe<T> {
    pub time: Time,
    pub value: T,
    #[serde(default)]
    pub easing: Easing,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Curve<T> {
    pub keyframes: Vec<Keyframe<T>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Property<T> {
    Static(T),
    Animated(Curve<T>),
}

impl<T: Interpolate> Property<T> {
    pub fn at(&self, time: Time) -> T {
        match self {
            Self::Static(value) => value.clone(),
            Self::Animated(curve) => curve.at(time),
        }
    }
}

impl<T: Interpolate> Curve<T> {
    /// Endpoints, interior lerp, clamp outside the keyframe span.
    pub fn at(&self, time: Time) -> T {
        let frames = &self.keyframes;
        match frames.len() {
            0 => panic!("curve has no keyframes"),
            1 => return frames[0].value.clone(),
            _ => {}
        }
        if time <= frames[0].time {
            return frames[0].value.clone();
        }
        let last = frames.last().expect("non-empty");
        if time >= last.time {
            return last.value.clone();
        }
        for pair in frames.windows(2) {
            let left = &pair[0];
            let right = &pair[1];
            if time >= left.time && time <= right.time {
                let span = right.time.checked_sub(left.time).unwrap_or(Time::ZERO);
                if span.is_zero() {
                    return right.value.clone();
                }
                let delta = time.checked_sub(left.time).unwrap_or(Time::ZERO);
                let num = delta.num().saturating_mul(span.den());
                let den = delta.den().saturating_mul(span.num());
                return T::interpolate(&left.value, &right.value, num, den);
            }
        }
        last.value.clone()
    }
}

fn lerp_i64(a: i64, b: i64, num: i64, den: i64) -> i64 {
    if den == 0 {
        return b;
    }
    let num = num.clamp(0, den);
    let delta = i128::from(b) - i128::from(a);
    let scaled = delta
        .saturating_mul(i128::from(num))
        .checked_div(i128::from(den))
        .unwrap_or(0);
    i64::try_from(i128::from(a) + scaled).unwrap_or(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sec(n: i64) -> Time {
        Time::seconds(n)
    }

    fn curve(points: &[(i64, u8)]) -> Curve<u8> {
        Curve {
            keyframes: points
                .iter()
                .map(|(t, v)| Keyframe {
                    time: sec(*t),
                    value: *v,
                    easing: Easing::Linear,
                })
                .collect(),
        }
    }

    #[test]
    fn endpoints_and_midpoint() {
        let p = Property::Animated(curve(&[(0, 0), (4, 100)]));
        assert_eq!(p.at(sec(0)), 0);
        assert_eq!(p.at(sec(4)), 100);
        assert_eq!(p.at(sec(2)), 50);
    }

    #[test]
    fn clamps_outside_span() {
        let p = Property::Animated(curve(&[(1, 10), (3, 30)]));
        assert_eq!(p.at(sec(0)), 10);
        assert_eq!(p.at(sec(9)), 30);
    }

    #[test]
    fn static_ignores_time() {
        let p = Property::Static(80u8);
        assert_eq!(p.at(sec(0)), 80);
        assert_eq!(p.at(sec(99)), 80);
    }
}
