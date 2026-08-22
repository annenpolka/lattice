use std::path::{Path, PathBuf};
use std::process::Command;

use lattice_core::{Time, TimeError};
use serde::Deserialize;
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

/// Backend-neutral media metadata. Engine/Studio must not parse ffprobe JSON.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaInfo {
    pub duration: Time,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_video: bool,
    pub has_audio: bool,
    pub frame_rate_num: Option<i64>,
    pub frame_rate_den: Option<i64>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct FfprobeJson {
    format: Option<FfprobeFormat>,
    #[serde(default)]
    streams: Vec<FfprobeStream>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
}

/// Probe container duration. Uses millisecond rounding of ffprobe's decimal.
pub fn probe_duration(path: impl AsRef<Path>) -> Result<Time, ProbeError> {
    Ok(probe_media(path)?.duration)
}

/// Probe duration, video, and audio metadata via ffprobe JSON.
pub fn probe_media(path: impl AsRef<Path>) -> Result<MediaInfo, ProbeError> {
    let output = Command::new(ffprobe_bin())
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type,width,height,r_frame_rate,avg_frame_rate,sample_rate,channels",
            "-of",
            "json",
        ])
        .arg(path.as_ref())
        .output()?;
    if !output.status.success() {
        return Err(ProbeError::Ffprobe {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let parsed: FfprobeJson =
        serde_json::from_slice(&output.stdout).map_err(|err| ProbeError::Parse(err.to_string()))?;
    media_info_from_ffprobe(parsed)
}

fn media_info_from_ffprobe(parsed: FfprobeJson) -> Result<MediaInfo, ProbeError> {
    let duration_text = parsed
        .format
        .and_then(|format| format.duration)
        .ok_or_else(|| ProbeError::Parse("missing format.duration".into()))?;
    let duration = parse_ffprobe_seconds(&duration_text)?;
    let mut info = MediaInfo {
        duration,
        width: None,
        height: None,
        has_video: false,
        has_audio: false,
        frame_rate_num: None,
        frame_rate_den: None,
        sample_rate: None,
        channels: None,
    };
    for stream in parsed.streams {
        match stream.codec_type.as_deref() {
            Some("video") => {
                info.has_video = true;
                if info.width.is_none() {
                    info.width = stream.width;
                    info.height = stream.height;
                    let rate = stream
                        .avg_frame_rate
                        .as_deref()
                        .filter(|rate| *rate != "0/0")
                        .or(stream.r_frame_rate.as_deref());
                    if let Some((num, den)) = rate.and_then(parse_frame_rate) {
                        info.frame_rate_num = Some(num);
                        info.frame_rate_den = Some(den);
                    }
                }
            }
            Some("audio") => {
                info.has_audio = true;
                if info.sample_rate.is_none() {
                    info.sample_rate = stream.sample_rate.as_deref().and_then(|s| s.parse().ok());
                    info.channels = stream.channels;
                }
            }
            _ => {}
        }
    }
    Ok(info)
}

fn parse_frame_rate(text: &str) -> Option<(i64, i64)> {
    let (num, den) = text.split_once('/')?;
    let num: i64 = num.parse().ok()?;
    let den: i64 = den.parse().ok()?;
    if den == 0 {
        return None;
    }
    Some((num, den))
}

pub(crate) fn parse_ffprobe_seconds(text: &str) -> Result<Time, ProbeError> {
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

/// True when the bottom title bar of a PPM frame is yellow.
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

/// Count near-white pixels in a horizontal band (title glyphs).
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn near_white_pixels(
    ppm: impl AsRef<Path>,
    y0_frac: f32,
    y1_frac: f32,
) -> Result<u32, ProbeError> {
    let bytes = std::fs::read(ppm.as_ref())?;
    let image = parse_ppm6(&bytes).ok_or_else(|| ProbeError::Parse("not a P6 PPM".into()))?;
    if image.height == 0 || image.width == 0 {
        return Ok(0);
    }
    let y0 = ((image.height as f32 * y0_frac) as u32).min(image.height.saturating_sub(1));
    let y1 = ((image.height as f32 * y1_frac) as u32)
        .max(y0 + 1)
        .min(image.height);
    let mut count = 0u32;
    for y in y0..y1 {
        for x in 0..image.width {
            let [r, g, b] = image.pixel(x, y);
            if r > 210 && g > 210 && b > 210 {
                count += 1;
            }
        }
    }
    Ok(count)
}

/// Pixel rows excluding the title band, for freeze-hold identity checks.
pub fn content_pixels(ppm: impl AsRef<Path>) -> Result<Vec<u8>, ProbeError> {
    let bytes = std::fs::read(ppm.as_ref())?;
    let image = parse_ppm6(&bytes).ok_or_else(|| ProbeError::Parse("not a P6 PPM".into()))?;
    let bar = 40.min(image.height / 3).max(8);
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

/// Mean absolute sample value of a little-endian s16le mono stream.
pub fn pcm_rms(bytes: &[u8]) -> u64 {
    if bytes.len() < 2 {
        return 0;
    }
    let mut sum: u128 = 0;
    let mut n: u128 = 0;
    for chunk in bytes.chunks_exact(2) {
        let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
        sum += u128::from(sample.unsigned_abs());
        n += 1;
    }
    u64::try_from(sum.checked_div(n).unwrap_or(0)).unwrap_or(u64::MAX)
}

/// Extract interleaved s16le mono PCM from a media file.
pub fn extract_pcm_s16le(path: impl AsRef<Path>) -> Result<Vec<u8>, ProbeError> {
    extract_pcm_s16le_span(path, None)
}

pub fn extract_pcm_s16le_span(
    path: impl AsRef<Path>,
    duration: Option<Time>,
) -> Result<Vec<u8>, ProbeError> {
    let mut command = Command::new(crate::export::ffmpeg_bin());
    command.args(["-y", "-i"]).arg(path.as_ref());
    if let Some(duration) = duration {
        command.args(["-t", &crate::export::ffmpeg_seconds(duration)]);
    }
    command.args(["-vn", "-ac", "1", "-ar", "8000", "-f", "s16le", "-"]);
    let output = command.output()?;
    if !output.status.success() {
        return Err(ProbeError::Ffprobe {
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(output.stdout)
}

pub fn has_audio_stream(path: impl AsRef<Path>) -> Result<bool, ProbeError> {
    Ok(probe_media(path)?.has_audio)
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

/// Last-resort system TTF/OTF lookup. Production prefers project-local / lock / fixture.
pub fn find_font() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("LATTICE_FONT") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let candidates = [
        r"C:\Windows\Fonts\arial.ttf",
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\calibri.ttf",
        r"C:\Windows\Fonts\tahoma.ttf",
        r"C:\Windows\Fonts\consola.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/Library/Fonts/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_eleven_point_five_from_ffprobe() {
        let t = parse_ffprobe_seconds("11.500000").unwrap();
        assert_eq!(t, Time::from_decimal_seconds(11, 5, 1).unwrap());
    }

    #[test]
    fn parses_frame_rate() {
        assert_eq!(parse_frame_rate("10/1"), Some((10, 1)));
        assert_eq!(parse_frame_rate("0/0"), None);
    }
}
