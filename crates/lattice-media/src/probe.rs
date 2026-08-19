use std::path::Path;
use std::process::Command;

use lattice_core::{Time, TimeError};
use thiserror::Error;

use crate::export::ffprobe_bin;

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("failed to run ffprobe: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("ffprobe failed (status {status}): {stderr}")]
    Ffprobe { status: String, stderr: String },
    #[error("ffprobe duration was not a number: {0}")]
    Parse(String),
    #[error(transparent)]
    Time(#[from] TimeError),
}

/// Probe container duration. Uses millisecond rounding of ffprobe's decimal.
pub fn probe_duration(path: impl AsRef<Path>) -> Result<Time, ProbeError> {
    let output = Command::new(ffprobe_bin())
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path.as_ref())
        .output()?;
    if !output.status.success() {
        return Err(ProbeError::Ffprobe {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_ffprobe_seconds(&text)
}

fn parse_ffprobe_seconds(text: &str) -> Result<Time, ProbeError> {
    let (whole_text, frac_text) = text.split_once('.').unwrap_or((text, "0"));
    let whole: i64 = whole_text
        .parse()
        .map_err(|_| ProbeError::Parse(text.to_string()))?;
    let digits: String = frac_text.chars().filter(char::is_ascii_digit).collect();
    let mut padded = digits;
    while padded.len() < 3 {
        padded.push('0');
    }
    let millis_part: i64 = padded[..3]
        .parse()
        .map_err(|_| ProbeError::Parse(text.to_string()))?;
    let round_up = padded.as_bytes().get(3).is_some_and(|d| *d >= b'5');
    let millis = whole
        .checked_mul(1000)
        .and_then(|n| n.checked_add(millis_part))
        .and_then(|n| n.checked_add(i64::from(round_up)))
        .ok_or_else(|| ProbeError::Parse(text.to_string()))?;
    Ok(Time::milliseconds(millis))
}

/// True when the bottom title bar of a PPM frame is yellow (drawbox overlay).
pub fn title_bar_present(ppm: impl AsRef<Path>) -> Result<bool, ProbeError> {
    let bytes = std::fs::read(ppm.as_ref())?;
    let image = parse_ppm6(&bytes).ok_or_else(|| ProbeError::Parse("not a P6 PPM".into()))?;
    if image.height < 4 || image.width == 0 {
        return Ok(false);
    }
    let y = image.height - 2;
    let mut yellow = 0u32;
    for x in 0..image.width {
        let [r, g, b] = image.pixel(x, y);
        if r > 200 && g > 200 && b < 40 {
            yellow += 1;
        }
    }
    Ok(yellow * 2 > image.width)
}

/// Pixel rows excluding the title bar, for freeze-hold identity checks.
pub fn content_pixels(ppm: impl AsRef<Path>) -> Result<Vec<u8>, ProbeError> {
    let bytes = std::fs::read(ppm.as_ref())?;
    let image = parse_ppm6(&bytes).ok_or_else(|| ProbeError::Parse("not a P6 PPM".into()))?;
    let bar = 8.min(image.height);
    let rows = image.height.saturating_sub(bar);
    let n = (rows * image.width * 3) as usize;
    Ok(image.data[..n].to_vec())
}

pub fn mean_abs_diff(left: &[u8], right: &[u8]) -> u64 {
    let n = left.len().min(right.len());
    if n == 0 {
        return u64::MAX;
    }
    let sum: u64 = left
        .iter()
        .zip(right)
        .map(|(a, b)| u64::from(a.abs_diff(*b)))
        .sum();
    sum / n as u64
}

struct Ppm {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Ppm {
    fn pixel(&self, x: u32, y: u32) -> [u8; 3] {
        let i = ((y * self.width + x) * 3) as usize;
        [self.data[i], self.data[i + 1], self.data[i + 2]]
    }
}

fn parse_ppm6(bytes: &[u8]) -> Option<Ppm> {
    if !bytes.starts_with(b"P6") {
        return None;
    }
    let mut rest = &bytes[2..];
    rest = skip_ws_and_comments(rest);
    let (width, rest) = take_u32(rest)?;
    let rest = skip_ws_and_comments(rest);
    let (height, rest) = take_u32(rest)?;
    let rest = skip_ws_and_comments(rest);
    let (max, rest) = take_u32(rest)?;
    if max != 255 {
        return None;
    }
    let data = rest
        .strip_prefix(b"\n")
        .or_else(|| rest.strip_prefix(b" "))?;
    let expected = width as usize * height as usize * 3;
    if data.len() < expected {
        return None;
    }
    Some(Ppm {
        width,
        height,
        data: data[..expected].to_vec(),
    })
}

fn skip_ws_and_comments(mut bytes: &[u8]) -> &[u8] {
    loop {
        while bytes.first().is_some_and(u8::is_ascii_whitespace) {
            bytes = &bytes[1..];
        }
        if bytes.first() == Some(&b'#') {
            while bytes.first().is_some_and(|b| *b != b'\n') {
                bytes = &bytes[1..];
            }
            continue;
        }
        break;
    }
    bytes
}

fn take_u32(bytes: &[u8]) -> Option<(u32, &[u8])> {
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    let n = std::str::from_utf8(&bytes[..i]).ok()?.parse().ok()?;
    Some((n, &bytes[i..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_eleven_point_five_from_ffprobe() {
        let t = parse_ffprobe_seconds("11.500000").unwrap();
        assert_eq!(t, Time::from_decimal_seconds(11, 5, 1).unwrap());
    }
}
