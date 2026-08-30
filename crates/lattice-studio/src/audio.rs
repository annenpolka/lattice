//! AudioPlan-backed Studio monitoring.
//!
//! Export and monitoring deliberately share [`lattice_engine::Engine::prepare_audio`],
//! so generated speech, clip gain, and commentary ducking have one implementation.
//! The output device is only a PCM sink: it never reads an exported movie.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lattice_engine::{
    AudioMixReport, Engine, EngineError, MixSpec, PcmBuffer, PreparedAudio, Project, ResolveLock,
    Time, audio_plan_from_timeline, flatten_project, source_revision,
};
use thiserror::Error;

use crate::StudioSession;

/// PCM format negotiated with the output device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioDeviceFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioDeviceFormat {
    fn validate(self) -> Result<Self, AudioDeviceInitError> {
        if self.sample_rate == 0 || self.channels == 0 {
            return Err(AudioDeviceInitError::InvalidFormat {
                sample_rate: self.sample_rate,
                channels: self.channels,
            });
        }
        Ok(self)
    }
}

/// A fully decoded/mixed `AudioPlan` ready for the device callback.
///
/// Preparing this value may run `FFmpeg` decoders and is synchronous. Studio
/// should prepare it on a worker, then call [`AudioMonitor::load`] on the UI
/// thread. `None` from [`Self::prepare`] means the flattened timeline contains
/// no audio windows. Once a window exists, resolution/decode failures are
/// returned rather than converted to silence.
#[derive(Clone, Debug)]
pub struct AudioProgram {
    pcm: Arc<PcmBuffer>,
    report: AudioMixReport,
    peak: f32,
}

impl AudioProgram {
    /// Convenience snapshot + prepare path. This remains synchronous; callers
    /// that cannot block should snapshot [`AudioPrepareJob`] on the UI thread
    /// and move the job into a worker before calling [`Self::prepare_job`].
    pub fn prepare(
        session: &StudioSession,
        format: AudioDeviceFormat,
    ) -> Result<Option<Self>, AudioPrepareError> {
        Self::prepare_job(&session.request_audio_prepare_job(), format)
    }

    /// Decode/mix a cloneable, immutable session snapshot.
    pub fn prepare_job(
        job: &AudioPrepareJob,
        format: AudioDeviceFormat,
    ) -> Result<Option<Self>, AudioPrepareError> {
        if !job.has_audio_windows()? {
            return Ok(None);
        }
        let format = format.validate().map_err(AudioPrepareError::DeviceFormat)?;
        // Pass an explicit captured lock, including an empty lock. `None` would
        // let Engine reload a newer lock and violate the job snapshot.
        let prepared = Engine::default().prepare_audio(
            &job.project,
            &job.media_root,
            Some(&job.lock),
            MixSpec {
                sample_rate: format.sample_rate,
                channels: format.channels,
            },
        )?;
        prepared.map(Self::from_prepared).transpose()
    }

    fn from_prepared(prepared: PreparedAudio) -> Result<Self, AudioPrepareError> {
        let report = prepared.report().clone();
        let pcm = prepared.into_pcm();
        validate_pcm(&pcm, &report)?;
        let peak = pcm
            .samples
            .iter()
            .copied()
            .map(f32::abs)
            .fold(0.0, f32::max);
        Ok(Self {
            pcm: Arc::new(pcm),
            report,
            peak,
        })
    }

    pub fn format(&self) -> AudioDeviceFormat {
        AudioDeviceFormat {
            sample_rate: self.pcm.sample_rate,
            channels: self.pcm.channels,
        }
    }

    pub fn duration(&self) -> Time {
        self.report.duration
    }

    pub fn frame_count(&self) -> u64 {
        u64::try_from(self.pcm.frame_count()).unwrap_or(u64::MAX)
    }

    pub fn peak(&self) -> f32 {
        self.peak
    }

    pub fn report(&self) -> &AudioMixReport {
        &self.report
    }
}

/// Immutable input for asynchronous `AudioPlan` decoding/mixing.
///
/// The project is the session's current in-memory compilation, so unsaved VEL
/// edits are preserved. The lock is captured at request time. `stamp` lets the
/// UI reject a worker result after either source or lock state changes.
#[derive(Clone, Debug)]
pub struct AudioPrepareJob {
    project: Project,
    media_root: PathBuf,
    lock: ResolveLock,
    stamp: String,
}

impl AudioPrepareJob {
    pub fn stamp(&self) -> &str {
        &self.stamp
    }

    pub fn media_root(&self) -> &Path {
        &self.media_root
    }

    /// Cheap preflight used before touching an output device. This makes an
    /// `AudioPlan` with no windows a successful `None` even on headless hosts.
    pub fn has_audio_windows(&self) -> Result<bool, AudioPrepareError> {
        let timeline = flatten_project(&self.project).map_err(EngineError::from)?;
        Ok(!audio_plan_from_timeline(&timeline).windows.is_empty())
    }
}

impl StudioSession {
    /// Snapshot the current unsaved project and resolve lock for a PCM worker.
    pub fn request_audio_prepare_job(&self) -> AudioPrepareJob {
        let media_root = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        AudioPrepareJob {
            project: self.compilation.project.clone(),
            lock: Engine::load_lock(&media_root).unwrap_or_default(),
            stamp: self.audio_prepare_stamp(),
            media_root,
        }
    }

    /// Current source+lock identity for latest-wins worker result acceptance.
    pub fn audio_prepare_stamp(&self) -> String {
        let media_root = self.path.parent().unwrap_or_else(|| Path::new("."));
        let lock_stamp = std::fs::read(media_root.join("lattice.lock.json"))
            .ok()
            .map_or_else(
                || "nolock".into(),
                |bytes| source_revision(&String::from_utf8_lossy(&bytes)),
            );
        format!(
            "{}:{}:{lock_stamp}",
            self.path.display(),
            source_revision(&self.compilation.source)
        )
    }
}

fn validate_pcm(pcm: &PcmBuffer, report: &AudioMixReport) -> Result<(), AudioPrepareError> {
    if pcm.sample_rate == 0 || pcm.channels == 0 {
        return Err(AudioPrepareError::InvalidPcm {
            reason: format!(
                "zero format ({} Hz, {} channel(s))",
                pcm.sample_rate, pcm.channels
            ),
        });
    }
    if !pcm.samples.len().is_multiple_of(usize::from(pcm.channels)) {
        return Err(AudioPrepareError::InvalidPcm {
            reason: "interleaved sample count is not channel-aligned".into(),
        });
    }
    if pcm.samples.iter().any(|sample| !sample.is_finite()) {
        return Err(AudioPrepareError::InvalidPcm {
            reason: "PCM contains a non-finite sample".into(),
        });
    }
    if report.window_count == 0 {
        return Err(AudioPrepareError::InvalidPcm {
            reason: "prepared PCM has no AudioPlan windows; expected no program".into(),
        });
    }
    if report.sample_rate != pcm.sample_rate
        || report.channels != pcm.channels
        || report.frame_count != pcm.frame_count()
    {
        return Err(AudioPrepareError::InvalidPcm {
            reason: "mix report does not match the PCM buffer".into(),
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum AudioPrepareError {
    #[error(transparent)]
    Engine(#[from] EngineError),
    #[error("invalid output-device format: {0}")]
    DeviceFormat(AudioDeviceInitError),
    #[error("invalid prepared AudioPlan PCM: {reason}")]
    InvalidPcm { reason: String },
}

/// Stable error category independent of CPAL's platform error representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioDeviceFailureKind {
    Busy,
    Changed,
    Unavailable,
    HostUnavailable,
    InvalidInput,
    PermissionDenied,
    RealtimeDenied,
    ResourceExhausted,
    StreamInvalidated,
    Unsupported,
    Xrun,
    Backend,
    Other,
}

impl fmt::Display for AudioDeviceFailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Busy => "busy",
            Self::Changed => "changed",
            Self::Unavailable => "unavailable",
            Self::HostUnavailable => "host unavailable",
            Self::InvalidInput => "invalid input",
            Self::PermissionDenied => "permission denied",
            Self::RealtimeDenied => "realtime denied",
            Self::ResourceExhausted => "resource exhausted",
            Self::StreamInvalidated => "stream invalidated",
            Self::Unsupported => "unsupported",
            Self::Xrun => "xrun",
            Self::Backend => "backend",
            Self::Other => "other",
        })
    }
}

/// Failure while discovering or creating the output stream.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AudioDeviceInitError {
    #[error("Studio audio monitoring is unsupported on this platform")]
    UnsupportedPlatform,
    #[error("no default output device is available")]
    NoOutputDevice,
    #[error("invalid audio format: {sample_rate} Hz, {channels} channel(s)")]
    InvalidFormat { sample_rate: u32, channels: u16 },
    #[error("default output configuration failed ({kind}): {message}")]
    DefaultConfig {
        kind: AudioDeviceFailureKind,
        message: String,
    },
    #[error("output sample format `{sample_format}` is unsupported")]
    UnsupportedSampleFormat { sample_format: String },
    #[error(
        "output device format changed: prepared {prepared:?}, current device {device:?}; reprepare audio"
    )]
    FormatChanged {
        prepared: AudioDeviceFormat,
        device: AudioDeviceFormat,
    },
    #[error("output stream creation failed ({kind}): {message}")]
    BuildStream {
        kind: AudioDeviceFailureKind,
        message: String,
    },
    #[error("drift tolerance must be zero or positive (got {tolerance})")]
    InvalidDriftTolerance { tolerance: Time },
}

#[derive(Debug, Error)]
pub enum AudioOpenError {
    #[error(transparent)]
    Prepare(#[from] AudioPrepareError),
    #[error(transparent)]
    Device(#[from] AudioDeviceInitError),
    #[error("timeline AudioPlan disappeared while opening its output stream")]
    ProgramDisappeared,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioRuntimeStage {
    Play,
    Pause,
    Callback,
}

impl fmt::Display for AudioRuntimeStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Callback => "device callback",
        })
    }
}

/// A device failed after the stream was created, or the caller supplied a
/// playhead that cannot be the session's current position.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AudioRuntimeError {
    #[error("audio {stage} failed ({kind}): {message}")]
    Device {
        stage: AudioRuntimeStage,
        kind: AudioDeviceFailureKind,
        message: String,
    },
    #[error("audio playhead {playhead} is outside 0s..={duration}")]
    PlayheadOutOfRange { playhead: Time, duration: Time },
}

/// Drift threshold used while the Studio session is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioMonitorConfig {
    pub drift_tolerance: Time,
}

impl Default for AudioMonitorConfig {
    fn default() -> Self {
        Self {
            drift_tolerance: Time::milliseconds(80),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioTransportChange {
    None,
    Started,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioReposition {
    None,
    SessionPosition,
    DriftCorrection,
}

/// Observable relationship between the session clock and device PCM cursor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioSyncReport {
    pub expected_frame: u64,
    pub observed_frame: u64,
    pub resulting_frame: u64,
    pub drift_frames: i64,
    pub drift_micros: i64,
    pub transport: AudioTransportChange,
    pub reposition: AudioReposition,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioMonitorStatus {
    pub format: AudioDeviceFormat,
    pub duration: Time,
    pub frame_count: u64,
    pub cursor_frame: u64,
    pub playing: bool,
    pub peak: f32,
    pub last_sync: Option<AudioSyncReport>,
}

/// Narrow sink boundary used by the transport controller and its deterministic tests.
pub trait AudioOutput {
    fn format(&self) -> AudioDeviceFormat;
    fn cursor_frames(&self) -> u64;
    fn seek_frame(&mut self, frame: u64);
    fn play(&mut self) -> Result<(), AudioRuntimeError>;
    fn pause(&mut self) -> Result<(), AudioRuntimeError>;
    fn take_runtime_error(&mut self) -> Option<AudioRuntimeError>;
}

/// Session-clock-driven PCM monitor.
///
/// Device time is diagnostic only. Every transport entry point accepts the
/// authoritative [`StudioSession::playhead`] value; no method advances the
/// session from the device cursor.
pub struct AudioMonitor {
    program: AudioProgram,
    output: Box<dyn AudioOutput>,
    drift_tolerance_frames: u64,
    playing: bool,
    last_sync: Option<AudioSyncReport>,
}

impl AudioMonitor {
    /// Query the format a worker should pass to [`AudioProgram::prepare`].
    pub fn output_format() -> Result<AudioDeviceFormat, AudioDeviceInitError> {
        platform::output_format()
    }

    /// Attach already-prepared PCM to the current default output device.
    pub fn load(
        program: AudioProgram,
        config: AudioMonitorConfig,
    ) -> Result<Self, AudioDeviceInitError> {
        let output = platform::load(&program.pcm, program.format())?;
        Self::with_output(program, output, config)
    }

    /// Synchronous convenience path. UI code should prefer
    /// `output_format` -> worker `AudioProgram::prepare` -> `load`.
    pub fn open(
        session: &StudioSession,
        config: AudioMonitorConfig,
    ) -> Result<Option<Self>, AudioOpenError> {
        let job = session.request_audio_prepare_job();
        if !job.has_audio_windows()? {
            return Ok(None);
        }
        let format = Self::output_format()?;
        let Some(program) = AudioProgram::prepare_job(&job, format)? else {
            return Err(AudioOpenError::ProgramDisappeared);
        };
        Ok(Some(Self::load(program, config)?))
    }

    /// Custom output hook for deterministic tests and future device selection.
    pub fn with_output(
        program: AudioProgram,
        output: Box<dyn AudioOutput>,
        config: AudioMonitorConfig,
    ) -> Result<Self, AudioDeviceInitError> {
        if config.drift_tolerance < Time::ZERO {
            return Err(AudioDeviceInitError::InvalidDriftTolerance {
                tolerance: config.drift_tolerance,
            });
        }
        let prepared = program.format();
        let device = output.format().validate()?;
        if prepared != device {
            return Err(AudioDeviceInitError::FormatChanged { prepared, device });
        }
        let drift_tolerance_frames = time_to_frame_ceil(config.drift_tolerance, device.sample_rate);
        Ok(Self {
            program,
            output,
            drift_tolerance_frames,
            playing: false,
            last_sync: None,
        })
    }

    pub fn play(&mut self, session_playhead: Time) -> Result<AudioSyncReport, AudioRuntimeError> {
        self.sync(session_playhead, true)
    }

    pub fn pause(&mut self, session_playhead: Time) -> Result<AudioSyncReport, AudioRuntimeError> {
        self.sync(session_playhead, false)
    }

    pub fn seek(&mut self, session_playhead: Time) -> Result<AudioSyncReport, AudioRuntimeError> {
        self.surface_device_error()?;
        let expected = self.frame_for_playhead(session_playhead)?;
        let observed = self.output.cursor_frames();
        self.output.seek_frame(expected);
        let report = self.report(
            expected,
            observed,
            AudioTransportChange::None,
            AudioReposition::SessionPosition,
        );
        self.last_sync = Some(report);
        Ok(report)
    }

    /// Reconcile the sink with the current session transport. Call this after
    /// every Studio playback tick and immediately after play/pause/scrub.
    pub fn sync(
        &mut self,
        session_playhead: Time,
        session_playing: bool,
    ) -> Result<AudioSyncReport, AudioRuntimeError> {
        self.surface_device_error()?;
        let expected = self.frame_for_playhead(session_playhead)?;
        let observed = self.output.cursor_frames();
        let drift = signed_diff(observed, expected);
        let mut transport = AudioTransportChange::None;
        let mut reposition = AudioReposition::None;

        if session_playing {
            if !self.playing {
                if observed != expected {
                    self.output.seek_frame(expected);
                    reposition = AudioReposition::SessionPosition;
                }
                self.output.play()?;
                self.playing = true;
                transport = AudioTransportChange::Started;
            } else if drift.unsigned_abs() > self.drift_tolerance_frames {
                self.output.seek_frame(expected);
                reposition = AudioReposition::DriftCorrection;
            }
        } else {
            if self.playing {
                self.output.pause()?;
                self.playing = false;
                transport = AudioTransportChange::Paused;
            }
            if self.output.cursor_frames() != expected {
                self.output.seek_frame(expected);
                reposition = AudioReposition::SessionPosition;
            }
        }

        let report = self.report(expected, observed, transport, reposition);
        self.last_sync = Some(report);
        Ok(report)
    }

    pub fn take_runtime_error(&mut self) -> Option<AudioRuntimeError> {
        self.output.take_runtime_error()
    }

    pub fn status(&self) -> AudioMonitorStatus {
        AudioMonitorStatus {
            format: self.program.format(),
            duration: self.program.duration(),
            frame_count: self.program.frame_count(),
            cursor_frame: self.output.cursor_frames(),
            playing: self.playing,
            peak: self.program.peak(),
            last_sync: self.last_sync,
        }
    }

    pub fn program(&self) -> &AudioProgram {
        &self.program
    }

    fn surface_device_error(&mut self) -> Result<(), AudioRuntimeError> {
        match self.output.take_runtime_error() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn frame_for_playhead(&self, playhead: Time) -> Result<u64, AudioRuntimeError> {
        if playhead < Time::ZERO || playhead > self.program.duration() {
            return Err(AudioRuntimeError::PlayheadOutOfRange {
                playhead,
                duration: self.program.duration(),
            });
        }
        Ok(
            time_to_frame_floor(playhead, self.program.format().sample_rate)
                .min(self.program.frame_count()),
        )
    }

    fn report(
        &self,
        expected: u64,
        observed: u64,
        transport: AudioTransportChange,
        reposition: AudioReposition,
    ) -> AudioSyncReport {
        let drift_frames = signed_diff(observed, expected);
        AudioSyncReport {
            expected_frame: expected,
            observed_frame: observed,
            resulting_frame: self.output.cursor_frames(),
            drift_frames,
            drift_micros: frames_to_micros(drift_frames, self.program.format().sample_rate),
            transport,
            reposition,
        }
    }
}

fn time_to_frame_floor(time: Time, sample_rate: u32) -> u64 {
    let numerator = i128::from(time.num()).saturating_mul(i128::from(sample_rate));
    let denominator = i128::from(time.den()).max(1);
    u64::try_from(numerator.div_euclid(denominator).max(0)).unwrap_or(u64::MAX)
}

fn time_to_frame_ceil(time: Time, sample_rate: u32) -> u64 {
    time.frame_count_ceil(i64::from(sample_rate), 1)
        .unwrap_or(u64::MAX)
}

fn signed_diff(left: u64, right: u64) -> i64 {
    let difference = i128::from(left) - i128::from(right);
    i64::try_from(difference).unwrap_or(if difference.is_negative() {
        i64::MIN
    } else {
        i64::MAX
    })
}

fn frames_to_micros(frames: i64, sample_rate: u32) -> i64 {
    if sample_rate == 0 {
        return 0;
    }
    let micros = i128::from(frames).saturating_mul(1_000_000) / i128::from(sample_rate);
    i64::try_from(micros).unwrap_or(if micros.is_negative() {
        i64::MIN
    } else {
        i64::MAX
    })
}

#[cfg(any(windows, target_os = "macos"))]
mod platform {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, SampleFormat, SizedSample};

    use super::{
        AudioDeviceFailureKind, AudioDeviceFormat, AudioDeviceInitError, AudioOutput,
        AudioRuntimeError, AudioRuntimeStage, PcmBuffer,
    };

    struct DeviceBuilder {
        device: cpal::Device,
        config: cpal::StreamConfig,
        sample_format: SampleFormat,
    }

    impl DeviceBuilder {
        fn open() -> Result<Self, AudioDeviceInitError> {
            let host = cpal::default_host();
            let device = host
                .default_output_device()
                .ok_or(AudioDeviceInitError::NoOutputDevice)?;
            let supported = device.default_output_config().map_err(|error| {
                AudioDeviceInitError::DefaultConfig {
                    kind: failure_kind(error.kind()),
                    message: error.to_string(),
                }
            })?;
            let sample_format = supported.sample_format();
            if !matches!(
                sample_format,
                SampleFormat::I8
                    | SampleFormat::I16
                    | SampleFormat::I24
                    | SampleFormat::I32
                    | SampleFormat::I64
                    | SampleFormat::U8
                    | SampleFormat::U16
                    | SampleFormat::U24
                    | SampleFormat::U32
                    | SampleFormat::U64
                    | SampleFormat::F32
                    | SampleFormat::F64
            ) {
                return Err(AudioDeviceInitError::UnsupportedSampleFormat {
                    sample_format: sample_format.to_string(),
                });
            }
            let config = supported.config();
            AudioDeviceFormat {
                sample_rate: config.sample_rate,
                channels: config.channels,
            }
            .validate()?;
            Ok(Self {
                device,
                config,
                sample_format,
            })
        }

        fn format(&self) -> AudioDeviceFormat {
            AudioDeviceFormat {
                sample_rate: self.config.sample_rate,
                channels: self.config.channels,
            }
        }

        fn build(self, pcm: &Arc<PcmBuffer>) -> Result<CpalOutput, AudioDeviceInitError> {
            let cursor = Arc::new(AtomicU64::new(0));
            let seek_epoch = Arc::new(AtomicU64::new(0));
            let runtime_error = Arc::new(Mutex::new(None));
            let stream = match self.sample_format {
                SampleFormat::I8 => {
                    self.build_typed::<i8>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::I16 => {
                    self.build_typed::<i16>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::I24 => {
                    self.build_typed::<cpal::I24>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::I32 => {
                    self.build_typed::<i32>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::I64 => {
                    self.build_typed::<i64>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::U8 => {
                    self.build_typed::<u8>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::U16 => {
                    self.build_typed::<u16>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::U24 => {
                    self.build_typed::<cpal::U24>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::U32 => {
                    self.build_typed::<u32>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::U64 => {
                    self.build_typed::<u64>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::F32 => {
                    self.build_typed::<f32>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                SampleFormat::F64 => {
                    self.build_typed::<f64>(pcm, &cursor, &seek_epoch, &runtime_error)
                }
                unsupported => {
                    return Err(AudioDeviceInitError::UnsupportedSampleFormat {
                        sample_format: unsupported.to_string(),
                    });
                }
            }
            .map_err(|error| AudioDeviceInitError::BuildStream {
                kind: failure_kind(error.kind()),
                message: error.to_string(),
            })?;
            Ok(CpalOutput {
                stream,
                format: self.format(),
                cursor,
                seek_epoch,
                runtime_error,
                frame_count: u64::try_from(pcm.frame_count()).unwrap_or(u64::MAX),
            })
        }

        fn build_typed<T>(
            &self,
            pcm: &Arc<PcmBuffer>,
            cursor: &Arc<AtomicU64>,
            seek_epoch: &Arc<AtomicU64>,
            runtime_error: &Arc<Mutex<Option<AudioRuntimeError>>>,
        ) -> Result<cpal::Stream, cpal::Error>
        where
            T: SizedSample + FromSample<f32>,
        {
            let pcm = Arc::clone(pcm);
            let cursor = Arc::clone(cursor);
            let seek_epoch = Arc::clone(seek_epoch);
            let device_error = Arc::clone(runtime_error);
            self.device.build_output_stream(
                self.config,
                move |output: &mut [T], _| {
                    write_pcm(output, &pcm, &cursor, &seek_epoch);
                },
                move |error| {
                    if let Ok(mut slot) = device_error.lock() {
                        *slot = Some(AudioRuntimeError::Device {
                            stage: AudioRuntimeStage::Callback,
                            kind: failure_kind(error.kind()),
                            message: error.to_string(),
                        });
                    }
                },
                None,
            )
        }
    }

    struct CpalOutput {
        stream: cpal::Stream,
        format: AudioDeviceFormat,
        cursor: Arc<AtomicU64>,
        seek_epoch: Arc<AtomicU64>,
        runtime_error: Arc<Mutex<Option<AudioRuntimeError>>>,
        frame_count: u64,
    }

    impl AudioOutput for CpalOutput {
        fn format(&self) -> AudioDeviceFormat {
            self.format
        }

        fn cursor_frames(&self) -> u64 {
            self.cursor.load(Ordering::Acquire)
        }

        fn seek_frame(&mut self, frame: u64) {
            self.cursor
                .store(frame.min(self.frame_count), Ordering::Release);
            self.seek_epoch.fetch_add(1, Ordering::AcqRel);
        }

        fn play(&mut self) -> Result<(), AudioRuntimeError> {
            self.stream
                .play()
                .map_err(|error| AudioRuntimeError::Device {
                    stage: AudioRuntimeStage::Play,
                    kind: failure_kind(error.kind()),
                    message: error.to_string(),
                })
        }

        fn pause(&mut self) -> Result<(), AudioRuntimeError> {
            self.stream
                .pause()
                .map_err(|error| AudioRuntimeError::Device {
                    stage: AudioRuntimeStage::Pause,
                    kind: failure_kind(error.kind()),
                    message: error.to_string(),
                })
        }

        fn take_runtime_error(&mut self) -> Option<AudioRuntimeError> {
            self.runtime_error.lock().ok()?.take()
        }
    }

    fn write_pcm<T>(output: &mut [T], pcm: &PcmBuffer, cursor: &AtomicU64, seek_epoch: &AtomicU64)
    where
        T: SizedSample + FromSample<f32>,
    {
        let channels = usize::from(pcm.channels.max(1));
        let start_frame = cursor.load(Ordering::Acquire);
        let epoch = seek_epoch.load(Ordering::Acquire);
        let start_sample = usize::try_from(start_frame)
            .unwrap_or(usize::MAX)
            .saturating_mul(channels);
        for (offset, target) in output.iter_mut().enumerate() {
            let sample = pcm
                .samples
                .get(start_sample.saturating_add(offset))
                .copied()
                .unwrap_or(0.0);
            *target = T::from_sample(sample);
        }
        let supplied_frames = u64::try_from(output.len() / channels).unwrap_or(u64::MAX);
        let end_frame = start_frame
            .saturating_add(supplied_frames)
            .min(u64::try_from(pcm.frame_count()).unwrap_or(u64::MAX));
        if seek_epoch.load(Ordering::Acquire) == epoch {
            let _ = cursor.compare_exchange(
                start_frame,
                end_frame,
                Ordering::AcqRel,
                Ordering::Acquire,
            );
        }
    }

    pub(super) fn output_format() -> Result<AudioDeviceFormat, AudioDeviceInitError> {
        Ok(DeviceBuilder::open()?.format())
    }

    pub(super) fn load(
        pcm: &Arc<PcmBuffer>,
        prepared: AudioDeviceFormat,
    ) -> Result<Box<dyn AudioOutput>, AudioDeviceInitError> {
        let builder = DeviceBuilder::open()?;
        let device = builder.format();
        if device != prepared {
            return Err(AudioDeviceInitError::FormatChanged { prepared, device });
        }
        Ok(Box::new(builder.build(pcm)?))
    }

    fn failure_kind(kind: cpal::ErrorKind) -> AudioDeviceFailureKind {
        match kind {
            cpal::ErrorKind::DeviceBusy => AudioDeviceFailureKind::Busy,
            cpal::ErrorKind::DeviceChanged => AudioDeviceFailureKind::Changed,
            cpal::ErrorKind::DeviceNotAvailable => AudioDeviceFailureKind::Unavailable,
            cpal::ErrorKind::HostUnavailable => AudioDeviceFailureKind::HostUnavailable,
            cpal::ErrorKind::InvalidInput => AudioDeviceFailureKind::InvalidInput,
            cpal::ErrorKind::PermissionDenied => AudioDeviceFailureKind::PermissionDenied,
            cpal::ErrorKind::RealtimeDenied => AudioDeviceFailureKind::RealtimeDenied,
            cpal::ErrorKind::ResourceExhausted => AudioDeviceFailureKind::ResourceExhausted,
            cpal::ErrorKind::StreamInvalidated => AudioDeviceFailureKind::StreamInvalidated,
            cpal::ErrorKind::UnsupportedConfig | cpal::ErrorKind::UnsupportedOperation => {
                AudioDeviceFailureKind::Unsupported
            }
            cpal::ErrorKind::Xrun => AudioDeviceFailureKind::Xrun,
            cpal::ErrorKind::BackendError => AudioDeviceFailureKind::Backend,
            _ => AudioDeviceFailureKind::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use super::{
        AudioDeviceFailureKind, AudioDeviceFormat, AudioDeviceInitError, AudioMixReport,
        AudioMonitor, AudioMonitorConfig, AudioOutput, AudioProgram, AudioReposition,
        AudioRuntimeError, AudioRuntimeStage, AudioTransportChange, PcmBuffer, ResolveLock,
        StudioSession, Time,
    };

    #[derive(Default)]
    struct FakeState {
        cursor: u64,
        play_calls: usize,
        pause_calls: usize,
        runtime_error: Option<AudioRuntimeError>,
    }

    struct FakeOutput {
        format: AudioDeviceFormat,
        state: Arc<Mutex<FakeState>>,
    }

    impl AudioOutput for FakeOutput {
        fn format(&self) -> AudioDeviceFormat {
            self.format
        }

        fn cursor_frames(&self) -> u64 {
            self.state.lock().unwrap().cursor
        }

        fn seek_frame(&mut self, frame: u64) {
            self.state.lock().unwrap().cursor = frame;
        }

        fn play(&mut self) -> Result<(), AudioRuntimeError> {
            self.state.lock().unwrap().play_calls += 1;
            Ok(())
        }

        fn pause(&mut self) -> Result<(), AudioRuntimeError> {
            self.state.lock().unwrap().pause_calls += 1;
            Ok(())
        }

        fn take_runtime_error(&mut self) -> Option<AudioRuntimeError> {
            self.state.lock().unwrap().runtime_error.take()
        }
    }

    fn test_program(value: f32) -> AudioProgram {
        test_program_for_format(
            value,
            AudioDeviceFormat {
                sample_rate: 1_000,
                channels: 2,
            },
        )
    }

    fn test_program_for_format(value: f32, format: AudioDeviceFormat) -> AudioProgram {
        let frame_count = usize::try_from(format.sample_rate).unwrap() * 2;
        AudioProgram {
            pcm: Arc::new(PcmBuffer {
                sample_rate: format.sample_rate,
                channels: format.channels,
                samples: vec![value; frame_count * usize::from(format.channels)],
            }),
            report: AudioMixReport {
                duration: Time::seconds(2),
                sample_rate: format.sample_rate,
                channels: format.channels,
                frame_count,
                window_count: 1,
                hold_window_count: 0,
                generated_window_count: 0,
                decoded_sources: vec!["capture".into()],
            },
            peak: value.abs(),
        }
    }

    fn monitor(value: f32) -> (AudioMonitor, Arc<Mutex<FakeState>>) {
        let state = Arc::new(Mutex::new(FakeState::default()));
        let output = FakeOutput {
            format: AudioDeviceFormat {
                sample_rate: 1_000,
                channels: 2,
            },
            state: Arc::clone(&state),
        };
        let monitor = AudioMonitor::with_output(
            test_program(value),
            Box::new(output),
            AudioMonitorConfig::default(),
        )
        .unwrap();
        (monitor, state)
    }

    #[test]
    fn session_position_controls_play_pause_seek_and_drift_correction() {
        let (mut monitor, state) = monitor(0.25);

        let started = monitor.play(Time::milliseconds(500)).unwrap();
        assert_eq!(started.expected_frame, 500);
        assert_eq!(started.observed_frame, 0);
        assert_eq!(started.resulting_frame, 500);
        assert_eq!(started.transport, AudioTransportChange::Started);
        assert_eq!(started.reposition, AudioReposition::SessionPosition);
        assert_eq!(state.lock().unwrap().play_calls, 1);

        state.lock().unwrap().cursor = 540;
        let within_tolerance = monitor.sync(Time::milliseconds(500), true).unwrap();
        assert_eq!(within_tolerance.drift_frames, 40);
        assert_eq!(within_tolerance.reposition, AudioReposition::None);
        assert_eq!(state.lock().unwrap().cursor, 540);

        state.lock().unwrap().cursor = 700;
        let corrected = monitor.sync(Time::milliseconds(500), true).unwrap();
        assert_eq!(corrected.drift_frames, 200);
        assert_eq!(corrected.drift_micros, 200_000);
        assert_eq!(corrected.reposition, AudioReposition::DriftCorrection);
        assert_eq!(corrected.resulting_frame, 500);

        state.lock().unwrap().cursor = 510;
        let paused = monitor.pause(Time::milliseconds(750)).unwrap();
        assert_eq!(paused.transport, AudioTransportChange::Paused);
        assert_eq!(paused.reposition, AudioReposition::SessionPosition);
        assert_eq!(state.lock().unwrap().pause_calls, 1);
        assert_eq!(state.lock().unwrap().cursor, 750);

        let sought = monitor.seek(Time::milliseconds(1_250)).unwrap();
        assert_eq!(sought.expected_frame, 1_250);
        assert_eq!(sought.resulting_frame, 1_250);
        assert_eq!(sought.reposition, AudioReposition::SessionPosition);
        assert!(!monitor.status().playing);
    }

    #[test]
    fn callback_failure_is_typed_and_never_hidden_as_silence() {
        let (mut monitor, state) = monitor(0.25);
        let expected = AudioRuntimeError::Device {
            stage: AudioRuntimeStage::Callback,
            kind: AudioDeviceFailureKind::Unavailable,
            message: "headphones disconnected".into(),
        };
        state.lock().unwrap().runtime_error = Some(expected.clone());

        assert_eq!(monitor.sync(Time::ZERO, false).unwrap_err(), expected);
        assert!(monitor.take_runtime_error().is_none());
    }

    #[test]
    fn silent_but_valid_plan_remains_an_observable_program() {
        let (monitor, _) = monitor(0.0);
        let status = monitor.status();
        assert!(status.peak.abs() < f32::EPSILON);
        assert_eq!(status.frame_count, 2_000);
        assert_eq!(monitor.program().report().window_count, 1);
    }

    #[test]
    fn invalid_playhead_and_format_mismatch_are_typed() {
        let (mut monitor, _) = monitor(0.25);
        assert_eq!(
            monitor.seek(Time::seconds(3)).unwrap_err(),
            AudioRuntimeError::PlayheadOutOfRange {
                playhead: Time::seconds(3),
                duration: Time::seconds(2),
            }
        );

        let state = Arc::new(Mutex::new(FakeState::default()));
        let output = FakeOutput {
            format: AudioDeviceFormat {
                sample_rate: 48_000,
                channels: 2,
            },
            state,
        };
        assert!(matches!(
            AudioMonitor::with_output(
                test_program(0.25),
                Box::new(output),
                AudioMonitorConfig::default()
            ),
            Err(AudioDeviceInitError::FormatChanged { .. })
        ));
    }

    #[test]
    fn prepare_job_captures_unsaved_source_and_lock_stamp() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("lattice-audio-job-{nanos}"));
        std::fs::create_dir_all(&dir).unwrap();
        let vel = dir.join("main.vel");
        std::fs::copy(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/gameplay-commentary/main.vel"),
            &vel,
        )
        .unwrap();
        let mut session = StudioSession::open(&vel).unwrap();

        let before = session.request_audio_prepare_job();
        let edited = session
            .source()
            .replace("title \"Hello\"", "title \"Unsaved\"");
        session.set_working_source(edited).unwrap();
        let unsaved = session.request_audio_prepare_job();
        assert_ne!(before.stamp(), unsaved.stamp());
        assert_eq!(unsaved.stamp(), session.audio_prepare_stamp());

        std::fs::write(
            dir.join("lattice.lock.json"),
            serde_json::to_vec(&ResolveLock::default()).unwrap(),
        )
        .unwrap();
        assert_ne!(unsaved.stamp(), session.audio_prepare_stamp());
        let locked = session.request_audio_prepare_job();
        assert_eq!(locked.stamp(), session.audio_prepare_stamp());
        assert_eq!(locked.lock, ResolveLock::default());
        assert_eq!(locked.project, unsaved.project);

        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn no_audio_windows_need_no_device_format_or_pcm() {
        let compilation = lattice_engine::Engine::default()
            .compile(
                r#"project "silent"
sequence main { demo }
scene demo {
  title "Silent" { at 0s for 1s }
}
"#,
            )
            .unwrap();
        let job = super::AudioPrepareJob {
            project: compilation.project,
            media_root: PathBuf::from("."),
            lock: ResolveLock::default(),
            stamp: "silent".into(),
        };
        assert!(!job.has_audio_windows().unwrap());
        assert!(
            AudioProgram::prepare_job(
                &job,
                AudioDeviceFormat {
                    sample_rate: 0,
                    channels: 0,
                }
            )
            .unwrap()
            .is_none()
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires a logged-in macOS session with a default output device"]
    fn macos_default_output_stream_plays_and_pauses_silence() {
        let format = AudioMonitor::output_format().expect("macOS default CoreAudio output");
        assert!(format.sample_rate > 0);
        assert!(format.channels > 0);

        let mut monitor = AudioMonitor::load(
            test_program_for_format(0.0, format),
            AudioMonitorConfig::default(),
        )
        .expect("build a CoreAudio output stream");
        monitor.play(Time::ZERO).expect("start CoreAudio stream");
        std::thread::sleep(std::time::Duration::from_millis(50));
        monitor.pause(Time::ZERO).expect("pause CoreAudio stream");
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod platform {
    use super::{AudioDeviceFormat, AudioDeviceInitError, AudioOutput, PcmBuffer};
    use std::sync::Arc;

    pub(super) fn output_format() -> Result<AudioDeviceFormat, AudioDeviceInitError> {
        Err(AudioDeviceInitError::UnsupportedPlatform)
    }

    pub(super) fn load(
        _pcm: &Arc<PcmBuffer>,
        _prepared: AudioDeviceFormat,
    ) -> Result<Box<dyn AudioOutput>, AudioDeviceInitError> {
        Err(AudioDeviceInitError::UnsupportedPlatform)
    }
}
