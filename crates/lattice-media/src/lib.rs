//! Media and render backends.
//!
//! `FFmpeg` is a decode/encode/mux adapter. Visual and audio semantics live in
//! `evaluate_at`, the Lattice compositor, and the PCM mixer.

mod audio;
mod backend;
mod composite;
mod decode;
mod encode;
mod export;
mod fixture;
mod font;
mod gpu;
mod mix;
mod plan;
mod preview;
mod probe;
mod sample;
mod text;

pub use audio::{AudioMixError, AudioMixReport, PreparedAudio, mix_timeline_audio};
pub use backend::{
    AudioRenderer, Encoder, FrameRenderer, OutputSpec, PcmBuffer, RawFrame, RendererBackend,
    RendererInitError, RendererInitStage, RendererRenderError, RendererRequest, RendererSelection,
    VideoDecoder,
};
pub use composite::CpuCompositor;
pub use encode::FfmpegEncoder;
pub use export::{
    ExportError, ExportReport, OutputSpecReport, PreviewOptions, export_preview, extract_frame,
    ffmpeg_bin, ffprobe_bin,
};
pub use fixture::{
    DEFAULT_SOURCE_DURATION_SECS, generate_av_fixture, generate_av_fixture_rate,
    generate_av_fixture_size, generate_test_source,
};
pub use font::{FontResolution, locked_font_asset, materialize_font_for_lock, resolve_font};
pub use gpu::{GpuCompositor, GpuRendererRuntime};
pub use mix::{MixSpec, mix_plan};
pub use plan::{
    AudioWindow, OverlayWindow, PlanSegment, RenderPlan, plan_from_timeline,
    plan_from_timeline_with_spec,
};
pub use preview::{PreviewFrameRequest, map_timeline_to_source, preview_frame};
pub use probe::{
    MediaInfo, ProbeError, content_pixels, extract_pcm_s16le, extract_pcm_s16le_span, find_font,
    has_audio_stream, mean_abs_diff, near_white_pixels, pcm_rms, probe_duration, probe_media,
    title_bar_present,
};
pub use sample::{
    PreviewSampler, SampleSession, SessionRebindError, SessionRebindRequirement, render_still,
    render_timeline, sample_frame, try_gpu_sample,
};
pub use text::{TextCacheLimits, TextCacheStats};

pub const PREVIEW_FPS_NUM: i64 = 10;
pub const PREVIEW_FPS_DEN: i64 = 1;
pub const PREVIEW_WIDTH: u32 = 320;
pub const PREVIEW_HEIGHT: u32 = 180;
