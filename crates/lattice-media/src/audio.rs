//! Prepared timeline audio shared by export and Studio monitoring.

use lattice_core::{AudioPlan, Time, Timeline, audio_plan_from_timeline};
use serde::Serialize;
use thiserror::Error;

use crate::backend::PcmBuffer;
use crate::decode::collect_audio_sources;
use crate::export::{ExportError, PreviewOptions};
use crate::mix::{MixSpec, mix_plan, time_to_frames};

/// Why timeline audio could not be prepared or addressed.
#[derive(Debug, Error)]
pub enum AudioMixError {
    #[error("invalid audio mix format: sample rate {sample_rate} Hz, {channels} channel(s)")]
    InvalidMixSpec { sample_rate: u32, channels: u16 },
    #[error("audio window {window_index} has no media source")]
    MissingWindowSource { window_index: usize },
    #[error("generated audio `{media_name}` has no readable speech artifact in lattice.lock.json")]
    MissingGeneratedAsset { media_name: String },
    #[error("audio source `{media_name}` is unavailable: {source}")]
    SourceUnavailable {
        media_name: String,
        generated: bool,
        #[source]
        source: Box<ExportError>,
    },
    #[error("audio source `{media_name}` decoded to no PCM frames")]
    EmptySource { media_name: String, generated: bool },
    #[error("timeline audio mix failed: {source}")]
    Mix {
        #[source]
        source: Box<ExportError>,
    },
    #[error("audio playhead {playhead} is outside 0s..={duration}")]
    PlayheadOutOfRange { playhead: Time, duration: Time },
}
