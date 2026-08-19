use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Mul, Sub};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Rational time in seconds. Never a float.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct Time {
    num: i64,
    den: i64,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TimeError {
    #[error("time denominator must not be zero")]
    ZeroDenominator,
    #[error("time arithmetic overflowed")]
    Overflow,
    #[error("time is not an integer number of units")]
    NotInteger,
}

impl Time {
    pub const ZERO: Self = Self { num: 0, den: 1 };
    pub const ONE: Self = Self { num: 1, den: 1 };

    pub fn new(num: i64, den: i64) -> Result<Self, TimeError> {
        if den == 0 {
            return Err(TimeError::ZeroDenominator);
        }
        Ok(normalize(num, den))
    }

    pub fn seconds(seconds: i64) -> Self {
        normalize(seconds, 1)
    }

    pub fn milliseconds(ms: i64) -> Self {
        normalize(ms, 1000)
    }

    /// `whole + frac / 10^frac_digits` seconds, e.g. 5.2s → (5, 2, 1).
    pub fn from_decimal_seconds(
        whole: i64,
        frac: i64,
        frac_digits: u32,
    ) -> Result<Self, TimeError> {
        let den = 10i64.checked_pow(frac_digits).ok_or(TimeError::Overflow)?;
        let shifted = whole.checked_mul(den).ok_or(TimeError::Overflow)?;
        let signed_frac = if whole < 0 { -frac } else { frac };
        let num = shifted
            .checked_add(signed_frac)
            .ok_or(TimeError::Overflow)?;
        Self::new(num, den)
    }

    pub fn from_minutes_seconds(minutes: i64, seconds: Self) -> Result<Self, TimeError> {
        let mins = Self::seconds(minutes.checked_mul(60).ok_or(TimeError::Overflow)?);
        mins.checked_add(seconds)
    }

    pub fn from_frames(frames: i64, fps_num: i64, fps_den: i64) -> Result<Self, TimeError> {
        // frames / fps = frames * fps_den / fps_num seconds
        let num = frames.checked_mul(fps_den).ok_or(TimeError::Overflow)?;
        Self::new(num, fps_num)
    }

    pub const fn num(self) -> i64 {
        self.num
    }

    pub const fn den(self) -> i64 {
        self.den
    }

    pub const fn is_zero(self) -> bool {
        self.num == 0
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, TimeError> {
        cross(self, rhs, |a, b, d| a.checked_add(b).map(|n| (n, d)))
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, TimeError> {
        cross(self, rhs, |a, b, d| a.checked_sub(b).map(|n| (n, d)))
    }

    pub fn checked_mul(self, rhs: Self) -> Result<Self, TimeError> {
        let num = i128::from(self.num)
            .checked_mul(i128::from(rhs.num))
            .ok_or(TimeError::Overflow)?;
        let den = i128::from(self.den)
            .checked_mul(i128::from(rhs.den))
            .ok_or(TimeError::Overflow)?;
        from_i128(num, den)
    }

    /// `self * fps_num / fps_den` as an exact integer frame count.
    pub fn exact_frame_count(self, fps_num: i64, fps_den: i64) -> Result<u64, TimeError> {
        if fps_den == 0 || self.den == 0 {
            return Err(TimeError::ZeroDenominator);
        }
        let n = i128::from(self.num)
            .checked_mul(i128::from(fps_num))
            .ok_or(TimeError::Overflow)?;
        let d = i128::from(self.den)
            .checked_mul(i128::from(fps_den))
            .ok_or(TimeError::Overflow)?;
        if d == 0 {
            return Err(TimeError::ZeroDenominator);
        }
        if n % d != 0 {
            return Err(TimeError::NotInteger);
        }
        u64::try_from(n / d).map_err(|_| TimeError::Overflow)
    }

    pub fn saturating_cmp(self, other: Self) -> Ordering {
        let left = i128::from(self.num) * i128::from(other.den);
        let right = i128::from(other.num) * i128::from(self.den);
        left.cmp(&right)
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.saturating_cmp(*other)
    }
}

impl Add for Time {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).expect("time add overflow")
    }
}

impl Sub for Time {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).expect("time sub overflow")
    }
}

impl Mul for Time {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).expect("time mul overflow")
    }
}

impl fmt::Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.den == 1 {
            return write!(f, "{}s", self.num);
        }
        if self.den == 1000 {
            if self.num % 1000 == 0 {
                return write!(f, "{}s", self.num / 1000);
            }
            return write!(f, "{}ms", self.num);
        }
        // Prefer a short decimal when the denominator divides a power of 10.
        if let Some(text) = decimal_seconds(*self) {
            return write!(f, "{text}s");
        }
        write!(f, "{}/{}s", self.num, self.den)
    }
}

fn decimal_seconds(time: Time) -> Option<String> {
    for places in 1u32..=6 {
        let scale = 10i64.pow(places);
        if scale % time.den != 0 {
            continue;
        }
        let scaled = time.num * (scale / time.den);
        let whole = scaled / scale;
        let frac = (scaled % scale).unsigned_abs();
        let width = places as usize;
        let mut frac_text = format!("{frac:0width$}");
        while frac_text.ends_with('0') && frac_text.len() > 1 {
            frac_text.pop();
        }
        if frac == 0 {
            return Some(whole.to_string());
        }
        return Some(format!("{whole}.{frac_text}"));
    }
    None
}

fn normalize(num: i64, den: i64) -> Time {
    let (num, den) = if den < 0 { (-num, -den) } else { (num, den) };
    let g = i64::try_from(gcd(num.unsigned_abs(), den.unsigned_abs())).unwrap_or(1);
    Time {
        num: num / g,
        den: den / g,
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

fn cross(
    lhs: Time,
    rhs: Time,
    op: impl FnOnce(i128, i128, i128) -> Option<(i128, i128)>,
) -> Result<Time, TimeError> {
    let a = i128::from(lhs.num)
        .checked_mul(i128::from(rhs.den))
        .ok_or(TimeError::Overflow)?;
    let b = i128::from(rhs.num)
        .checked_mul(i128::from(lhs.den))
        .ok_or(TimeError::Overflow)?;
    let d = i128::from(lhs.den)
        .checked_mul(i128::from(rhs.den))
        .ok_or(TimeError::Overflow)?;
    let (num, den) = op(a, b, d).ok_or(TimeError::Overflow)?;
    from_i128(num, den)
}

fn from_i128(num: i128, den: i128) -> Result<Time, TimeError> {
    let num = i64::try_from(num).map_err(|_| TimeError::Overflow)?;
    let den = i64::try_from(den).map_err(|_| TimeError::Overflow)?;
    Time::new(num, den)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_five_point_two() {
        let t = Time::from_decimal_seconds(5, 2, 1).unwrap();
        assert_eq!(t, Time::new(26, 5).unwrap());
        assert_eq!(t.to_string(), "5.2s");
    }

    #[test]
    fn minutes_and_seconds() {
        let t = Time::from_minutes_seconds(26, Time::seconds(14)).unwrap();
        assert_eq!(t, Time::seconds(26 * 60 + 14));
    }

    #[test]
    fn add_is_exact() {
        let a = Time::from_decimal_seconds(1, 5, 1).unwrap();
        let b = Time::from_decimal_seconds(0, 7, 1).unwrap();
        assert_eq!(a + b, Time::from_decimal_seconds(2, 2, 1).unwrap());
    }
}

#[cfg(test)]
mod properties {
    use super::*;
    use proptest::prelude::*;

    fn arb_time() -> impl Strategy<Value = Time> {
        (-10_000i64..10_000, 1i64..1_000).prop_map(|(num, den)| Time::new(num, den).unwrap())
    }

    proptest! {
        #[test]
        fn add_commutative(a in arb_time(), b in arb_time()) {
            prop_assert_eq!(a.checked_add(b).ok(), b.checked_add(a).ok());
        }

        #[test]
        fn add_zero_identity(a in arb_time()) {
            prop_assert_eq!(a.checked_add(Time::ZERO).unwrap(), a);
        }
    }
}
