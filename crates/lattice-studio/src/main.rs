//! Lattice Studio entry. Always GPUI (`window` is the default crate feature).
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::ignored_unit_patterns,
    clippy::map_unwrap_or,
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::unreadable_literal,
    clippy::unused_self,
    clippy::useless_conversion
)]

use std::ops::Range;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Condvar, Mutex};

use gpui::{
    App, AppContext, Application, Bounds, ClipboardItem, Context, CursorStyle, Entity, FocusHandle,
    Focusable, InputHandler, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, ParentElement, Pixels, Render,
    RenderImage, ScrollDelta, ScrollHandle, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement, Styled, StyledImage, TextRun, Timer, TitlebarOptions,
    UTF16Selection, Window, WindowBounds, WindowOptions, canvas, div, img, px, rgb, size,
};
use lattice_engine::{
    Engine, OutputSpec, PreviewFrameRequest, PreviewOptions, PreviewSampler, RawFrame,
    RendererRequest, RendererSelection, Span,
};
use lattice_studio::audio::{
    AudioDeviceFormat, AudioMonitor, AudioMonitorConfig, AudioPrepareJob, AudioProgram,
    AudioReposition, AudioSyncReport, AudioTransportChange,
};
use lattice_studio::{
    CanvasPoint, CanvasRect, CanvasSize, CursorKind, PLAYBACK_TICK, PreviewInbox, ResizeCorner,
    StudioSession, UiFixture, playback_target, trace, write_geom_file, write_state_file,
};

#[cfg(test)]
mod ui_driver;

#[cfg(test)]
mod launch_tests {
    use super::{LaunchSpec, UiFixture, parse_launch};
    use std::ffi::OsString;

    fn args(items: &[&str]) -> Vec<OsString> {
        std::iter::once(OsString::from("lattice-studio"))
            .chain(items.iter().map(|item| OsString::from(*item)))
            .collect()
    }

    #[test]
    fn help_and_fixture_parse() {
        assert!(matches!(
            parse_launch(args(&["--help"])).unwrap(),
            LaunchSpec::Help
        ));
        assert!(matches!(
            parse_launch(args(&["--ui-fixture", "timeline-basic"])).unwrap(),
            LaunchSpec::Fixture(UiFixture::TimelineBasic)
        ));
        assert!(matches!(
            parse_launch(args(&["--ui-fixture=drag-valid"])).unwrap(),
            LaunchSpec::Fixture(UiFixture::DragValid)
        ));
        assert!(parse_launch(args(&["--ui-fixture", "timeline-basic", "main.vel"])).is_err());
        assert!(parse_launch(args(&["--unknown"])).is_err());
        assert!(matches!(
            parse_launch(args(&["/tmp/demo.vel"])).unwrap(),
            LaunchSpec::Vel(path) if path.ends_with("demo.vel")
        ));
    }

    #[test]
    fn windows_vel_path_launch_stays_a_single_path() {
        let windows_vel = parse_launch(args(&[r"C:\work\project\main.vel"])).unwrap();
        match windows_vel {
            LaunchSpec::Vel(path) => {
                let text = path.to_string_lossy();
                assert!(
                    text.contains("main.vel"),
                    "Windows VEL path must be preserved, got {text}"
                );
            }
            other => panic!("expected Vel, got {other:?}"),
        }
        assert!(matches!(
            parse_launch(args(&[])).unwrap(),
            LaunchSpec::DefaultDemo
        ));
        assert!(
            parse_launch(args(&[
                "--ui-fixture",
                "timeline-basic",
                r"C:\work\project\main.vel"
            ]))
            .is_err(),
            "Windows dogfood still passes a VEL path alone; fixture+path must stay rejected"
        );
        assert!(
            parse_launch(args(&["--ui-fixture", "timeline-basic"]))
                .unwrap()
                .vel_path()
                .is_ok()
        );
    }
}

fn main() -> ExitCode {
    let log_path = trace::install();
    let launch = match parse_launch(std::env::args_os()) {
        Ok(launch) => launch,
        Err(err) => {
            trace::log(format!("fatal: {err}"));
            return ExitCode::from(2);
        }
    };
    if matches!(launch, LaunchSpec::Help) {
        print_help();
        return ExitCode::SUCCESS;
    }
    let path = match launch.vel_path() {
        Ok(path) => path,
        Err(err) => {
            trace::log(format!("fatal: {err}"));
            return ExitCode::from(2);
        }
    };
    let fixture = launch.fixture_name();
    trace::log(format!(
        "start exe={} cwd={} vel={} fixture={} log={} preview={} autoplay={} smoke={} rustc={}",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".into()),
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".into()),
        path.display(),
        fixture.unwrap_or("none"),
        log_path.display(),
        if preview_extract_enabled() {
            "on"
        } else {
            "off"
        },
        if autoplay_enabled() { "on" } else { "off" },
        smoke_timeout_ms().map_or_else(|| "off".into(), |ms| format!("{ms}ms")),
        option_env!("CARGO_PKG_VERSION").unwrap_or("dev"),
    ));
    if let Err(err) = window_main(path, fixture.map(str::to_string)) {
        trace::log(format!("fatal: {err}"));
        return ExitCode::from(2);
    }
    trace::log("event loop returned (window closed)");
    ExitCode::SUCCESS
}

#[derive(Debug)]
enum LaunchSpec {
    Help,
    DefaultDemo,
    Vel(PathBuf),
    Fixture(UiFixture),
}

impl LaunchSpec {
    fn fixture_name(&self) -> Option<&'static str> {
        match self {
            Self::Fixture(fixture) => Some(fixture.as_str()),
            Self::Help | Self::DefaultDemo | Self::Vel(_) => None,
        }
    }

    fn vel_path(&self) -> Result<PathBuf, String> {
        match self {
            Self::Help => Err("help does not open a project".into()),
            Self::DefaultDemo => Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/warframe-cut/main.vel")),
            Self::Vel(path) => Ok(path.clone()),
            Self::Fixture(fixture) => fixture
                .materialize()
                .map_err(|err| format!("materialize --ui-fixture {fixture}: {err}")),
        }
    }
}

fn parse_launch(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<LaunchSpec, String> {
    let mut args = args.into_iter().skip(1);
    let mut fixture = None;
    let mut vel = None;
    let mut saw_help = false;
    while let Some(arg) = args.next() {
        let Some(text) = arg.to_str() else {
            if vel.is_some() {
                return Err("lattice-studio accepts one VEL path".into());
            }
            vel = Some(PathBuf::from(arg));
            continue;
        };
        if text == "--help" || text == "-h" {
            saw_help = true;
            continue;
        }
        if let Some(name) = text.strip_prefix("--ui-fixture=") {
            if fixture.is_some() {
                return Err("lattice-studio accepts one --ui-fixture".into());
            }
            fixture = Some(parse_ui_fixture(name)?);
            continue;
        }
        if text == "--ui-fixture" {
            if fixture.is_some() {
                return Err("lattice-studio accepts one --ui-fixture".into());
            }
            let name = args
                .next()
                .and_then(|value| value.into_string().ok())
                .ok_or_else(|| "missing --ui-fixture name".to_string())?;
            fixture = Some(parse_ui_fixture(&name)?);
            continue;
        }
        if text.starts_with('-') {
            return Err(format!(
                "unknown option `{text}`; see lattice-studio --help"
            ));
        }
        if vel.is_some() {
            return Err("lattice-studio accepts one VEL path".into());
        }
        vel = Some(PathBuf::from(text));
    }
    if saw_help {
        return Ok(LaunchSpec::Help);
    }
    match (fixture, vel) {
        (Some(_), Some(_)) => Err(
            "pass either a VEL path or --ui-fixture, not both (Windows dogfood still uses a path)"
                .into(),
        ),
        (Some(fixture), None) => Ok(LaunchSpec::Fixture(fixture)),
        (None, Some(path)) => Ok(LaunchSpec::Vel(path)),
        (None, None) => {
            if let Ok(name) = std::env::var("LATTICE_STUDIO_UI_FIXTURE")
                && !name.trim().is_empty()
            {
                return Ok(LaunchSpec::Fixture(parse_ui_fixture(&name)?));
            }
            Ok(LaunchSpec::DefaultDemo)
        }
    }
}

fn parse_ui_fixture(name: &str) -> Result<UiFixture, String> {
    UiFixture::parse(name).ok_or_else(|| {
        format!(
            "unknown --ui-fixture `{name}`; expected {}",
            UiFixture::ALL
                .iter()
                .map(|fixture| fixture.as_str())
                .collect::<Vec<_>>()
                .join("|")
        )
    })
}

fn print_help() {
    let names = UiFixture::ALL
        .iter()
        .map(|fixture| fixture.as_str())
        .collect::<Vec<_>>()
        .join("|");
    trace::log(format!(
        "usage: lattice-studio [VEL] | lattice-studio --ui-fixture <{names}>"
    ));
    trace::log(
        "Windows dogfood still opens a VEL path. --ui-fixture materializes a deterministic Engine/Studio project.",
    );
    trace::log(
        "LATTICE_STUDIO_PREVIEW=0 skips live frame extract; LATTICE_STUDIO_AUDIO_MONITOR=0 skips device output.",
    );
    trace::log("LATTICE_STUDIO_STATE writes the latest semantic_state JSON snapshot.");
}

fn window_main(path: PathBuf, fixture: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let renderer = studio_renderer_request()?;
    trace::log(format!("renderer requested={renderer}"));
    trace::log(format!("compile/open {}", path.display()));
    let session = StudioSession::open(&path).map_err(|err| {
        trace::log(format!("StudioSession::open failed: {err}"));
        err
    })?;
    let mut initial = session.semantic_state();
    if let Some(name) = &fixture {
        initial["fixture"] = serde_json::Value::String(name.clone());
        trace::log(format!("ui fixture={name}"));
    }
    initial["reason"] = serde_json::Value::String("open".into());
    trace::log(format!("semantic_state {initial}"));
    if let Err(err) = write_state_file(&initial) {
        trace::log(format!("semantic_state write failed: {err}"));
    }
    trace::log(format!(
        "open ok dirty={} playhead={:?} diagnostics={}",
        session.is_dirty(),
        session.playhead(),
        session.diagnostics().len()
    ));
    spawn_smoke_watchdog();
    trace::log("Application::run");
    Application::new().run(move |cx| {
        let bounds = Bounds::centered(None, size(px(1400.0), px(840.0)), cx);
        trace::log("open_window");
        match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(format!("Lattice Studio · {}", renderer_short(renderer)).into()),
                    appears_transparent: false,
                    ..Default::default()
                }),
                focus: true,
                show: true,
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
                    trace::log("StudioView::new");
                    let mut view = StudioView::new(
                        session,
                        renderer,
                        cx.focus_handle(),
                        cx.focus_handle(),
                        cx.focus_handle(),
                    );
                    view.ui_fixture = fixture;
                    view.adopt_locus_label();
                    view.spawn_preview_worker();
                    if audio_monitor_enabled() {
                        view.spawn_audio_worker();
                        view.queue_audio_prepare();
                    } else {
                        view.disable_audio_monitor();
                    }
                    view.spawn_play_clock(cx);
                    if autoplay_enabled() {
                        trace::log("autoplay");
                        view.start_play();
                    } else {
                        view.queue_preview();
                    }
                    view
                })
            },
        ) {
            Ok(_) => trace::log("open_window ok"),
            Err(err) => trace::log(format!("open_window failed: {err:?}")),
        }
        cx.activate(true);
        trace::log("activate");
    });
    Ok(())
}

fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(value) => {
            let value = value.to_ascii_lowercase();
            !matches!(value.as_str(), "0" | "off" | "false" | "no")
        }
        Err(_) => default,
    }
}

fn preview_extract_enabled() -> bool {
    env_flag("LATTICE_STUDIO_PREVIEW", true)
}

fn audio_monitor_enabled() -> bool {
    env_flag("LATTICE_STUDIO_AUDIO_MONITOR", true)
}

fn autoplay_enabled() -> bool {
    env_flag("LATTICE_STUDIO_AUTOPLAY", false)
}

fn studio_renderer_request() -> Result<RendererRequest, std::io::Error> {
    match std::env::var("LATTICE_STUDIO_RENDERER") {
        Err(_) => Ok(RendererRequest::RequireCpu),
        Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
            "cpu" | "require-cpu" | "require_cpu" => Ok(RendererRequest::RequireCpu),
            "dx12" | "gpu-dx12" | "gpu_dx12" | "require-gpu-dx12" | "require_gpu_dx12" => {
                Ok(RendererRequest::RequireGpuDx12)
            }
            other => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("invalid LATTICE_STUDIO_RENDERER `{other}`; expected `cpu` or `gpu-dx12`"),
            )),
        },
    }
}

fn renderer_short(renderer: RendererRequest) -> &'static str {
    match renderer {
        RendererRequest::RequireCpu => "CPU",
        RendererRequest::RequireGpuDx12 => "GPU DX12",
    }
}

fn smoke_timeout_ms() -> Option<u64> {
    let raw = std::env::var("LATTICE_STUDIO_SMOKE_MS").ok()?;
    let ms: u64 = raw.parse().ok()?;
    (ms > 0).then_some(ms)
}

/// Timed process exit for `scripts/studio-smoke.ps1`. Not a player shutdown path.
fn spawn_smoke_watchdog() {
    let Some(ms) = smoke_timeout_ms() else {
        return;
    };
    trace::log(format!("smoke timeout {ms}ms"));
    if let Err(err) = std::thread::Builder::new()
        .name("lattice-smoke".into())
        .spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(ms));
            trace::log("smoke quit");
            std::process::exit(0);
        })
    {
        trace::log(format!("smoke watchdog spawn failed: {err}"));
    }
}

struct StudioView {
    session: StudioSession,
    ui_fixture: Option<String>,
    title_draft: String,
    title_selection_utf16: Range<usize>,
    title_marked_utf16: Option<Range<usize>>,
    source_draft: String,
    source_selection_utf16: Range<usize>,
    source_marked_utf16: Option<Range<usize>>,
    source_error: Option<String>,
    last_render: Option<String>,
    focus: FocusHandle,
    title_focus: FocusHandle,
    source_focus: FocusHandle,
    source_scroll: ScrollHandle,
    first_paint_logged: bool,
    preview_shown: bool,
    rail_geom: Arc<Mutex<(f32, f32)>>,
    track_geoms: Arc<Mutex<Vec<(String, f32, f32, f32, f32)>>>,
    play_geom: Arc<Mutex<Option<(f32, f32, f32, f32)>>>,
    ruler_geom: Arc<Mutex<Option<(f32, f32, f32, f32)>>>,
    canvas_geom: Arc<Mutex<Option<(f32, f32, f32, f32)>>>,
    last_focused: Option<String>,
    last_inflight_key: Option<String>,
    geom_logged: bool,
    preview_dirty: bool,
    preview_slot: Arc<Mutex<PreviewSlot>>,
    preview_inbox: std::sync::Arc<PreviewInbox>,
    /// GPUI image lifetime is explicitly bounded to the displayed frame and its predecessor.
    preview_current: Option<Arc<RenderImage>>,
    preview_previous: Option<Arc<RenderImage>>,
    renderer: RendererRequest,
    renderer_selection: Option<RendererSelection>,
    renderer_error: Option<String>,
    preview_retry_required: bool,
    preview_sampler_reset_required: bool,
    hover_x: Option<f64>,
    hover_track: Option<String>,
    play_origin: Option<(std::time::Instant, lattice_engine::Time)>,
    last_preview_time: Option<lattice_engine::Time>,
    canvas_resize_corner: Option<ResizeCorner>,
    audio_inbox: Arc<AudioPrepareInbox>,
    audio_slot: Arc<Mutex<AudioPrepareSlot>>,
    audio_enabled: bool,
    audio_monitor_disabled: bool,
    audio_play_pending: bool,
    audio_generation: u64,
    audio_preparing: bool,
    audio_format: Option<AudioDeviceFormat>,
    audio_monitor: Option<AudioMonitor>,
    audio_program_stamp: Option<String>,
    audio_no_windows: bool,
    audio_error: Option<String>,
    audio_last_sync: Option<AudioSyncReport>,
}

#[derive(Clone)]
struct StudioTitleInputHandler {
    view: Entity<StudioView>,
    bounds: Bounds<Pixels>,
}

#[derive(Clone)]
struct StudioSourceInputHandler {
    view: Entity<StudioView>,
    bounds: Bounds<Pixels>,
}

fn clamp_utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
    let len = text.encode_utf16().count();
    let start = range.start.min(len);
    let end = range.end.min(len).max(start);
    start..end
}

fn byte_index_at_utf16(text: &str, offset: usize) -> usize {
    let mut units = 0;
    for (byte, ch) in text.char_indices() {
        if units >= offset {
            return byte;
        }
        units += ch.len_utf16();
        if units >= offset {
            return byte + ch.len_utf8();
        }
    }
    text.len()
}

fn utf16_range_to_bytes(text: &str, range: Range<usize>) -> Range<usize> {
    let range = clamp_utf16_range(text, range);
    byte_index_at_utf16(text, range.start)..byte_index_at_utf16(text, range.end)
}

fn byte_index_to_utf16(text: &str, byte_offset: usize) -> usize {
    let byte_offset = byte_offset.min(text.len());
    text[..byte_offset].chars().map(char::len_utf16).sum()
}

fn byte_range_to_utf16(text: &str, range: Range<usize>) -> Range<usize> {
    let start = range.start.min(text.len());
    let end = range.end.min(text.len()).max(start);
    byte_index_to_utf16(text, start)..byte_index_to_utf16(text, end)
}

fn previous_utf16_boundary(text: &str, offset: usize) -> usize {
    let byte = byte_index_at_utf16(text, offset);
    let previous = text[..byte]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index);
    byte_index_to_utf16(text, previous)
}

fn next_utf16_boundary(text: &str, offset: usize) -> usize {
    let byte = byte_index_at_utf16(text, offset);
    let next = text[byte..]
        .chars()
        .next()
        .map_or(text.len(), |ch| byte + ch.len_utf8());
    byte_index_to_utf16(text, next)
}

#[derive(Clone, Debug)]
struct SourceLine {
    number: usize,
    start: usize,
    end: usize,
    text: String,
}

fn source_lines(text: &str) -> Vec<SourceLine> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, part) in text.split_inclusive('\n').enumerate() {
        let line = part.strip_suffix('\n').unwrap_or(part);
        lines.push(SourceLine {
            number: index + 1,
            start,
            end: start + line.len(),
            text: line.to_string(),
        });
        start += part.len();
    }
    if lines.is_empty() || text.ends_with('\n') {
        lines.push(SourceLine {
            number: lines.len() + 1,
            start: text.len(),
            end: text.len(),
            text: String::new(),
        });
    }
    lines
}

fn source_line_index_at_byte(text: &str, byte_offset: usize) -> usize {
    text[..byte_offset.min(text.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
}

fn byte_ranges_intersect(a: Range<usize>, b: Range<usize>) -> bool {
    if a.is_empty() {
        return b.start <= a.start && a.start <= b.end;
    }
    a.start < b.end && b.start < a.end
}

impl InputHandler for StudioTitleInputHandler {
    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<UTF16Selection> {
        let range = self.view.read(cx).title_selection_utf16.clone();
        Some(UTF16Selection {
            range,
            reversed: false,
        })
    }

    fn marked_text_range(&mut self, _window: &mut Window, cx: &mut App) -> Option<Range<usize>> {
        self.view.read(cx).title_marked_utf16.clone()
    }

    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        let view = self.view.read(cx);
        let range_utf16 = clamp_utf16_range(&view.title_draft, range_utf16);
        let range = utf16_range_to_bytes(&view.title_draft, range_utf16.clone());
        *adjusted_range = Some(range_utf16);
        Some(view.title_draft[range].to_string())
    }

    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.view.update(cx, |view, cx| {
            let range_utf16 = replacement_range
                .or_else(|| view.title_marked_utf16.clone())
                .unwrap_or_else(|| view.title_selection_utf16.clone());
            let range_utf16 = clamp_utf16_range(&view.title_draft, range_utf16);
            let range = utf16_range_to_bytes(&view.title_draft, range_utf16.clone());
            view.title_draft.replace_range(range, text);
            let caret = range_utf16.start + text.encode_utf16().count();
            view.title_selection_utf16 = caret..caret;
            view.title_marked_utf16 = None;
            cx.notify();
        });
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.view.update(cx, |view, cx| {
            let range_utf16 = replacement_range
                .or_else(|| view.title_marked_utf16.clone())
                .unwrap_or_else(|| view.title_selection_utf16.clone());
            let range_utf16 = clamp_utf16_range(&view.title_draft, range_utf16);
            let range = utf16_range_to_bytes(&view.title_draft, range_utf16.clone());
            view.title_draft.replace_range(range, new_text);
            let inserted_len = new_text.encode_utf16().count();
            let inserted = range_utf16.start..range_utf16.start + inserted_len;
            view.title_marked_utf16 = (!new_text.is_empty()).then_some(inserted.clone());
            view.title_selection_utf16 = new_selected_range.map_or_else(
                || inserted.end..inserted.end,
                |selection| {
                    let start = (inserted.start + selection.start).min(inserted.end);
                    let end = (inserted.start + selection.end)
                        .min(inserted.end)
                        .max(start);
                    start..end
                },
            );
            cx.notify();
        });
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut App) {
        self.view.update(cx, |view, cx| {
            view.title_marked_utf16 = None;
            cx.notify();
        });
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        Some(self.bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        Some(self.view.read(cx).title_draft.encode_utf16().count())
    }
}

impl InputHandler for StudioSourceInputHandler {
    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.view.read(cx).source_selection_utf16.clone(),
            reversed: false,
        })
    }

    fn marked_text_range(&mut self, _window: &mut Window, cx: &mut App) -> Option<Range<usize>> {
        self.view.read(cx).source_marked_utf16.clone()
    }

    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<String> {
        let view = self.view.read(cx);
        let range_utf16 = clamp_utf16_range(&view.source_draft, range_utf16);
        let range = utf16_range_to_bytes(&view.source_draft, range_utf16.clone());
        *adjusted_range = Some(range_utf16);
        Some(view.source_draft[range].to_string())
    }

    fn replace_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        text: &str,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.view.update(cx, |view, cx| {
            let range = replacement_range
                .or_else(|| view.source_marked_utf16.clone())
                .unwrap_or_else(|| view.source_selection_utf16.clone());
            view.replace_source_range(range, text, None);
            view.commit_source_draft();
            cx.notify();
        });
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        replacement_range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut App,
    ) {
        self.view.update(cx, |view, cx| {
            let range = replacement_range
                .or_else(|| view.source_marked_utf16.clone())
                .unwrap_or_else(|| view.source_selection_utf16.clone());
            let inserted_len = new_text.encode_utf16().count();
            let marked_selection = new_selected_range.or(Some(inserted_len..inserted_len));
            view.replace_source_range(range, new_text, marked_selection);
            cx.notify();
        });
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut App) {
        self.view.update(cx, |view, cx| {
            view.source_marked_utf16 = None;
            view.commit_source_draft();
            cx.notify();
        });
    }

    fn bounds_for_range(
        &mut self,
        _range_utf16: Range<usize>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Option<Bounds<Pixels>> {
        Some(self.bounds)
    }

    fn character_index_for_point(
        &mut self,
        _point: gpui::Point<Pixels>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Option<usize> {
        Some(self.view.read(cx).source_selection_utf16.end)
    }
}

type PreviewWorkerResult = Result<(RawFrame, RendererSelection), String>;
type PreviewSlot = Option<(u64, PreviewWorkerResult, lattice_engine::Time, String)>;
type AudioPrepareResult = Result<Option<AudioProgram>, String>;
type AudioPrepareSlot = Option<(u64, String, AudioPrepareResult)>;

struct AudioPrepareRequest {
    generation: u64,
    stamp: String,
    job: AudioPrepareJob,
    format: AudioDeviceFormat,
}

struct AudioPrepareInbox {
    state: Mutex<AudioPrepareInboxState>,
    wake: Condvar,
}

struct AudioPrepareInboxState {
    pending: Option<AudioPrepareRequest>,
    stopped: bool,
}

impl AudioPrepareInbox {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(AudioPrepareInboxState {
                pending: None,
                stopped: false,
            }),
            wake: Condvar::new(),
        })
    }

    fn push(&self, request: AudioPrepareRequest) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.stopped {
            return;
        }
        state.pending = Some(request);
        self.wake.notify_one();
    }

    fn take_wait(&self) -> Option<AudioPrepareRequest> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if state.stopped {
                return None;
            }
            if let Some(request) = state.pending.take() {
                return Some(request);
            }
            state = self
                .wake
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    fn stop(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.stopped = true;
        state.pending = None;
        self.wake.notify_one();
    }
}

impl StudioView {
    fn new(
        session: StudioSession,
        renderer: RendererRequest,
        focus: FocusHandle,
        title_focus: FocusHandle,
        source_focus: FocusHandle,
    ) -> Self {
        let source_draft = session.source().to_string();
        Self {
            session,
            ui_fixture: None,
            title_draft: String::new(),
            title_selection_utf16: 0..0,
            title_marked_utf16: None,
            source_draft,
            source_selection_utf16: 0..0,
            source_marked_utf16: None,
            source_error: None,
            last_render: None,
            focus,
            title_focus,
            source_focus,
            source_scroll: ScrollHandle::new(),
            first_paint_logged: false,
            preview_shown: false,
            rail_geom: Arc::new(Mutex::new((0.0_f32, TIMELINE_WIDTH))),
            track_geoms: Arc::new(Mutex::new(Vec::new())),
            play_geom: Arc::new(Mutex::new(None)),
            ruler_geom: Arc::new(Mutex::new(None)),
            canvas_geom: Arc::new(Mutex::new(None)),
            last_focused: None,
            last_inflight_key: None,
            geom_logged: false,
            preview_dirty: true,
            preview_slot: Arc::new(Mutex::new(None)),
            preview_inbox: PreviewInbox::new(),
            preview_current: None,
            preview_previous: None,
            renderer,
            renderer_selection: None,
            renderer_error: None,
            preview_retry_required: false,
            preview_sampler_reset_required: false,
            hover_x: None,
            hover_track: None,
            play_origin: None,
            last_preview_time: None,
            canvas_resize_corner: None,
            audio_inbox: AudioPrepareInbox::new(),
            audio_slot: Arc::new(Mutex::new(None)),
            audio_enabled: false,
            audio_monitor_disabled: false,
            audio_play_pending: false,
            audio_generation: 0,
            audio_preparing: false,
            audio_format: None,
            audio_monitor: None,
            audio_program_stamp: None,
            audio_no_windows: false,
            audio_error: None,
            audio_last_sync: None,
        }
    }

    fn focused_name(&self, window: &Window) -> Option<&'static str> {
        if self.title_focus.is_focused(window) {
            Some("inspector.title")
        } else if self.source_focus.is_focused(window) {
            Some("vel.editor")
        } else if self.focus.is_focused(window) {
            Some("studio")
        } else {
            None
        }
    }

    fn refresh_focus(&mut self, window: &Window) {
        if let Some(focused) = self.focused_name(window) {
            self.last_focused = Some(focused.to_string());
        }
    }

    fn log_semantic_state(&mut self, reason: &str, window: Option<&Window>) {
        if let Some(window) = window {
            self.refresh_focus(window);
        }
        let mut state = self.session.semantic_state();
        if let Some(name) = &self.ui_fixture {
            state["fixture"] = serde_json::Value::String(name.clone());
        }
        state["reason"] = serde_json::Value::String(reason.into());
        if let Some(focused) = &self.last_focused {
            state["focused"] = serde_json::Value::String(focused.clone());
        }
        trace::log(format!("semantic_state {state}"));
        if let Err(err) = write_state_file(&state) {
            trace::log(format!("semantic_state write failed: {err}"));
        }
    }

    fn log_inflight_semantic_state(&mut self, reason: &str) {
        let state = self.session.semantic_state();
        let key = format!(
            "{reason}:{}:{}:{}",
            state["playhead"], state["interaction"], state["drag"]
        );
        if self.last_inflight_key.as_deref() == Some(key.as_str()) {
            return;
        }
        self.last_inflight_key = Some(key);
        self.log_semantic_state(reason, None);
    }

    fn maybe_log_smoke_geom(&mut self) {
        if self.geom_logged {
            return;
        }
        let play = self.play_geom.lock().ok().and_then(|slot| *slot);
        let ruler = self.ruler_geom.lock().ok().and_then(|slot| *slot);
        let tracks = self
            .track_geoms
            .lock()
            .ok()
            .map(|slots| slots.clone())
            .unwrap_or_default();
        let rail = self.rail_geom.lock().ok().map(|slot| *slot);
        let Some((play_x, play_y, play_w, play_h)) = play else {
            return;
        };
        let Some((ruler_x, ruler_y, ruler_w, ruler_h)) = ruler else {
            return;
        };
        if tracks.is_empty() {
            return;
        }
        let Some((rail_x, rail_w)) = rail else {
            return;
        };
        if rail_x <= 0.0 && rail_w <= 1.0 {
            return;
        }
        let tracks_json: Vec<serde_json::Value> = tracks
            .iter()
            .map(|(name, x, y, w, h)| {
                serde_json::json!({
                    "name": name,
                    "x": x,
                    "y": y,
                    "w": w,
                    "h": h,
                })
            })
            .collect();
        let geom = serde_json::json!({
            "play": { "x": play_x, "y": play_y, "w": play_w, "h": play_h },
            "ruler": { "x": ruler_x, "y": ruler_y, "w": ruler_w, "h": ruler_h },
            "rail": { "x": rail_x, "w": rail_w },
            "tracks": tracks_json,
        });
        trace::log(format!("smoke_geom {geom}"));
        if let Err(err) = write_geom_file(&geom) {
            trace::log(format!("smoke_geom write failed: {err}"));
        }
        self.geom_logged = true;
    }

    fn adopt_locus_label(&mut self) {
        if let Ok(Some(locus)) = self.session.current_locus() {
            self.title_draft = locus.label;
            let end = self.title_draft.encode_utf16().count();
            self.title_selection_utf16 = end..end;
            self.title_marked_utf16 = None;
        }
    }

    fn sync_source_draft(&mut self) {
        let source = self.session.source();
        if self.source_draft != source {
            self.source_draft = source.to_string();
            self.source_selection_utf16 =
                clamp_utf16_range(&self.source_draft, self.source_selection_utf16.clone());
            self.source_marked_utf16 = None;
        }
        self.source_error = None;
    }

    fn replace_source_range(
        &mut self,
        range_utf16: Range<usize>,
        text: &str,
        marked_selection: Option<Range<usize>>,
    ) {
        let range_utf16 = clamp_utf16_range(&self.source_draft, range_utf16);
        let byte_range = utf16_range_to_bytes(&self.source_draft, range_utf16.clone());
        self.source_draft.replace_range(byte_range, text);
        let inserted_len = text.encode_utf16().count();
        let inserted = range_utf16.start..range_utf16.start + inserted_len;
        self.source_marked_utf16 = marked_selection
            .as_ref()
            .filter(|_| !text.is_empty())
            .map(|_| inserted.clone());
        self.source_selection_utf16 = marked_selection.map_or_else(
            || inserted.end..inserted.end,
            |selection| {
                let start = (inserted.start + selection.start).min(inserted.end);
                let end = (inserted.start + selection.end)
                    .min(inserted.end)
                    .max(start);
                start..end
            },
        );
    }

    fn commit_source_draft(&mut self) {
        let source = self.source_draft.clone();
        let caret = byte_index_at_utf16(&source, self.source_selection_utf16.end);
        let locus_offset = caret.saturating_sub(1);
        match self.session.set_working_source(source) {
            Ok(()) => {
                self.source_error = None;
                if let Ok(offset) = u32::try_from(locus_offset) {
                    let _ = self.session.point_from_source_offset(offset);
                }
                self.adopt_locus_label();
                self.after_edit();
            }
            Err(err) => {
                trace::log(format!("VEL edit: {err}"));
                self.source_error = Some(err.to_string());
            }
        }
    }

    fn select_source_span(&mut self, span: Span) -> usize {
        self.sync_source_draft();
        let range = usize::try_from(span.start).unwrap_or(0)
            ..usize::try_from(span.end).unwrap_or(self.source_draft.len());
        self.source_selection_utf16 = byte_range_to_utf16(&self.source_draft, range.clone());
        self.source_marked_utf16 = None;
        source_line_index_at_byte(&self.source_draft, range.start)
    }

    fn spawn_play_clock(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(PLAYBACK_TICK).await;
                if this
                    .update(cx, |this, cx| {
                        let accepted = this.drain_preview();
                        let audio_changed = this.drain_audio_prepare();
                        if this.session.is_playing() {
                            let before = this.session.playhead();
                            this.catch_up_play_clock();
                            this.sync_audio_monitor("clock");
                            this.queue_preview_if_needed();
                            if accepted || audio_changed || this.session.playhead() != before {
                                cx.notify();
                            }
                        } else if this.preview_dirty {
                            this.sync_audio_monitor("clock-paused");
                            this.queue_preview_if_needed();
                            if accepted || audio_changed {
                                cx.notify();
                            }
                        } else if accepted
                            || audio_changed
                            || (!this.preview_shown
                                && (this.session.has_memory_preview()
                                    || this.session.peek_preview_frame().is_some()))
                        {
                            this.preview_shown = true;
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn catch_up_play_clock(&mut self) {
        let Some((started, origin)) = self.play_origin else {
            return;
        };
        let target = playback_target(origin, started.elapsed(), self.session.duration());
        let dt = target
            .checked_sub(self.session.playhead())
            .unwrap_or(lattice_engine::Time::ZERO);
        if dt > lattice_engine::Time::ZERO {
            self.session.step_clock(dt);
        }
        if !self.session.is_playing() {
            self.play_origin = None;
        }
    }

    fn spawn_preview_worker(&self) {
        let inbox = std::sync::Arc::clone(&self.preview_inbox);
        let slot = Arc::clone(&self.preview_slot);
        match std::thread::Builder::new()
            .name("lattice-preview".into())
            .spawn(move || preview_worker_loop(inbox, slot))
        {
            Ok(_) => trace::log("preview worker spawned"),
            Err(err) => trace::log(format!("preview worker spawn failed: {err}")),
        }
    }

    fn spawn_audio_worker(&mut self) {
        let inbox = Arc::clone(&self.audio_inbox);
        let slot = Arc::clone(&self.audio_slot);
        match std::thread::Builder::new()
            .name("lattice-audio-prepare".into())
            .spawn(move || audio_prepare_worker_loop(inbox, slot))
        {
            Ok(_) => {
                self.audio_enabled = true;
                trace::log("audio prepare worker spawned");
            }
            Err(err) => {
                self.audio_enabled = false;
                self.audio_error = Some(format!("audio worker spawn failed: {err}"));
                trace::log(format!("audio prepare worker spawn failed: {err}"));
            }
        }
    }

    fn disable_audio_monitor(&mut self) {
        self.audio_enabled = false;
        self.audio_monitor_disabled = true;
        self.audio_preparing = false;
        self.audio_monitor = None;
        self.audio_program_stamp = Some(self.session.audio_prepare_stamp());
        self.audio_no_windows = true;
        self.audio_error = None;
        self.audio_last_sync = None;
        trace::log("audio monitor explicitly disabled");
    }

    fn block_play_on_audio_error(&mut self, message: String, reason: &str) {
        if let Some(monitor) = self.audio_monitor.as_mut() {
            let _ = monitor.pause(self.session.playhead());
        }
        self.audio_monitor = None;
        self.audio_program_stamp = None;
        self.audio_preparing = false;
        self.audio_no_windows = false;
        self.audio_play_pending = false;
        self.audio_last_sync = None;
        self.audio_error = Some(message.clone());
        self.session.pause();
        self.play_origin = None;
        self.preview_inbox.clear_pending();
        self.preview_dirty = true;
        trace::log(format!("audio blocks A/V play reason={reason}: {message}"));
    }

    fn begin_ready_play(&mut self) {
        self.audio_play_pending = false;
        self.session.play();
        self.play_origin = Some((std::time::Instant::now(), self.session.playhead()));
        self.sync_audio_monitor("play");
        self.queue_preview();
        trace::log(format!(
            "play samples AudioPlan+video playhead={} {}",
            format_time(self.session.playhead()),
            self.audio_status()
        ));
        self.log_semantic_state("play", None);
    }

    fn queue_audio_prepare(&mut self) {
        // Headless VisualTestContext views intentionally do not spawn the
        // platform audio worker. This keeps selector tests deterministic and
        // prevents them from probing a developer or CI output device.
        if !self.audio_enabled {
            return;
        }
        let job = self.session.request_audio_prepare_job();
        let stamp = job.stamp().to_string();
        if self.audio_program_stamp.as_deref() == Some(stamp.as_str()) {
            return;
        }
        if self.audio_monitor.is_some() || self.audio_program_stamp.is_some() {
            let resume = self.session.is_playing() || self.audio_play_pending;
            if let Some(monitor) = self.audio_monitor.as_mut() {
                let _ = monitor.pause(self.session.playhead());
            }
            self.audio_monitor = None;
            self.audio_program_stamp = None;
            self.audio_no_windows = false;
            self.audio_last_sync = None;
            self.audio_play_pending = resume;
            self.session.pause();
            self.play_origin = None;
            trace::log("audio program stamp changed; old PCM detached");
        }
        match job.has_audio_windows() {
            Ok(false) => {
                self.audio_monitor = None;
                self.audio_program_stamp = Some(stamp);
                self.audio_preparing = false;
                self.audio_no_windows = true;
                self.audio_error = None;
                trace::log("audio plan has no windows");
                if self.audio_play_pending {
                    self.begin_ready_play();
                }
                return;
            }
            Ok(true) => {}
            Err(err) => {
                self.block_play_on_audio_error(
                    format!("audio plan inspect failed: {err}"),
                    "plan-inspect",
                );
                return;
            }
        }
        let format = match self.audio_format {
            Some(format) => format,
            None => match AudioMonitor::output_format() {
                Ok(format) => {
                    self.audio_format = Some(format);
                    format
                }
                Err(err) => {
                    self.block_play_on_audio_error(
                        format!("audio device init failed: {err}"),
                        "device-init",
                    );
                    return;
                }
            },
        };
        self.audio_generation = self.audio_generation.saturating_add(1);
        self.audio_preparing = true;
        self.audio_no_windows = false;
        self.audio_error = None;
        self.audio_inbox.push(AudioPrepareRequest {
            generation: self.audio_generation,
            stamp: stamp.clone(),
            job,
            format,
        });
        trace::log(format!(
            "audio prepare queued generation={} stamp={} format={}Hz/{}ch",
            self.audio_generation, stamp, format.sample_rate, format.channels
        ));
    }

    fn drain_audio_prepare(&mut self) -> bool {
        let result = self.audio_slot.lock().ok().and_then(|mut slot| slot.take());
        let Some((generation, stamp, result)) = result else {
            return false;
        };
        if generation != self.audio_generation || stamp != self.session.audio_prepare_stamp() {
            trace::log(format!(
                "audio prepare stale generation={generation} stamp={stamp}"
            ));
            if generation == self.audio_generation {
                self.audio_preparing = false;
                self.queue_audio_prepare();
            }
            return false;
        }
        self.audio_preparing = false;
        self.audio_monitor = None;
        self.audio_last_sync = None;
        match result {
            Ok(Some(program)) => {
                let report = program.report().clone();
                match AudioMonitor::load(program, AudioMonitorConfig::default()) {
                    Ok(monitor) => {
                        self.audio_monitor = Some(monitor);
                        self.audio_program_stamp = Some(stamp);
                        self.audio_no_windows = false;
                        self.audio_error = None;
                        trace::log(format!(
                            "audio ready frames={} windows={} generated={} sources={} peak-ready",
                            report.frame_count,
                            report.window_count,
                            report.generated_window_count,
                            report.decoded_sources.len()
                        ));
                        if self.audio_play_pending {
                            self.begin_ready_play();
                        } else {
                            self.sync_audio_monitor("prepared");
                        }
                    }
                    Err(err) => {
                        self.audio_format = None;
                        self.block_play_on_audio_error(
                            format!("audio stream load failed: {err}"),
                            "stream-load",
                        );
                    }
                }
            }
            Ok(None) => {
                self.audio_program_stamp = Some(stamp);
                self.audio_no_windows = true;
                self.audio_error = None;
                trace::log("audio prepare completed without windows");
                if self.audio_play_pending {
                    self.begin_ready_play();
                }
            }
            Err(err) => {
                self.block_play_on_audio_error(format!("audio prepare failed: {err}"), "prepare");
            }
        }
        true
    }

    fn sync_audio_monitor(&mut self, reason: &str) {
        let result = self
            .audio_monitor
            .as_mut()
            .map(|monitor| monitor.sync(self.session.playhead(), self.session.is_playing()));
        match result {
            Some(Ok(report)) => {
                let noteworthy = report.transport != AudioTransportChange::None
                    || report.reposition != AudioReposition::None;
                self.audio_last_sync = Some(report);
                if noteworthy {
                    trace::log(format!(
                        "audio sync reason={reason} transport={:?} reposition={:?} drift={}us frame={}/{}",
                        report.transport,
                        report.reposition,
                        report.drift_micros,
                        report.resulting_frame,
                        report.expected_frame
                    ));
                }
            }
            Some(Err(err)) => {
                self.audio_format = None;
                self.block_play_on_audio_error(format!("audio runtime failed: {err}"), reason);
            }
            None => {}
        }
    }

    fn invalidate_audio(&mut self, reason: &str) {
        if self.audio_monitor_disabled {
            self.audio_program_stamp = Some(self.session.audio_prepare_stamp());
            self.audio_no_windows = true;
            trace::log(format!("audio monitor disabled; skip invalidate {reason}"));
            return;
        }
        let resume = self.session.is_playing() || self.audio_play_pending;
        if let Some(monitor) = self.audio_monitor.as_mut() {
            let _ = monitor.pause(self.session.playhead());
        }
        self.audio_monitor = None;
        self.audio_program_stamp = None;
        self.audio_no_windows = false;
        self.audio_error = None;
        self.audio_last_sync = None;
        self.audio_play_pending = resume;
        self.audio_generation = self.audio_generation.saturating_add(1);
        self.session.pause();
        self.play_origin = None;
        trace::log(format!("audio invalidate {reason}"));
        self.queue_audio_prepare();
    }

    fn audio_status(&self) -> String {
        if self.audio_monitor_disabled {
            return "Audio · monitor explicitly disabled".into();
        }
        if let Some(error) = &self.audio_error {
            return format!("Audio error · {error}");
        }
        if self.audio_preparing {
            return if self.audio_play_pending {
                "Audio · preparing AudioPlan PCM · play pending".into()
            } else {
                "Audio · preparing AudioPlan PCM".into()
            };
        }
        if self.audio_no_windows {
            return "Audio · no timeline windows".into();
        }
        if let Some(monitor) = &self.audio_monitor {
            let status = monitor.status();
            // The detailed log keeps the pre-correction drift. The toolbar
            // reports the post-sync state so a successful seek/reprepare does
            // not look permanently out of sync.
            let drift = status.last_sync.map_or(0, |sync| {
                if sync.reposition == AudioReposition::None {
                    sync.drift_micros
                } else {
                    0
                }
            });
            return format!(
                "Audio · {} Hz / {} ch · drift {:+.1} ms · peak {:.2}",
                status.format.sample_rate,
                status.format.channels,
                drift as f64 / 1_000.0,
                status.peak
            );
        }
        "Audio · not initialized".into()
    }

    fn pointer_x(&self, window_x: gpui::Pixels) -> f64 {
        let (origin, width) = self
            .rail_geom
            .lock()
            .map(|g| *g)
            .unwrap_or((0.0, TIMELINE_WIDTH));
        let x = f32::from(window_x) - origin;
        let _ = width;
        f64::from(x)
    }

    fn begin_timeline_pointer_on(&mut self, window_x: gpui::Pixels, snap_off: bool, track: &str) {
        self.audio_play_pending = false;
        let x = self.pointer_x(window_x);
        self.apply_rail_width();
        if let Err(err) = self.session.begin_timeline_pointer_on(x, snap_off, track) {
            trace::log(format!("begin gesture: {err}"));
        } else {
            trace::log(format!(
                "gesture begin track={track} x={x:.1} kind={:?} playhead={}",
                self.session.gesture(),
                format_time(self.session.playhead())
            ));
        }
        self.play_origin = None;
        self.sync_audio_monitor("timeline-pointer");
        self.preview_inbox.clear_pending();
        self.preview_dirty = true;
        self.last_inflight_key = None;
        self.log_semantic_state("timeline-pointer-begin", None);
    }

    fn update_timeline_pointer(&mut self, window_x: gpui::Pixels, snap_off: bool) {
        if !self.session.gesture().is_active() {
            return;
        }
        let x = self.pointer_x(window_x);
        if let Err(err) = self.session.update_timeline_pointer(x, snap_off) {
            trace::log(format!("update gesture: {err}"));
        } else {
            trace::log(format!(
                "gesture update x={x:.1} playhead={}",
                format_time(self.session.playhead())
            ));
        }
        self.sync_audio_monitor("timeline-pointer-update");
        self.preview_dirty = true;
        self.log_inflight_semantic_state("timeline-pointer-update");
    }

    fn commit_timeline_pointer(&mut self, window_x: gpui::Pixels, snap_off: bool) {
        if !self.session.gesture().is_active() {
            return;
        }
        let x = self.pointer_x(window_x);
        match self.session.commit_timeline_pointer_snap(x, snap_off) {
            Ok(outcome) => {
                trace::log(format!(
                    "gesture commit x={x:.1} outcome={outcome:?} playhead={} undo={}",
                    format_time(self.session.playhead()),
                    self.session.undo_len()
                ));
                if outcome == lattice_studio::GestureOutcome::Applied {
                    self.invalidate_audio("timeline-edit");
                }
            }
            Err(err) => trace::log(format!("commit gesture: {err}")),
        }
        if let Some(err) = self.session.last_gesture_error() {
            trace::log(format!("gesture failed: {err}"));
        }
        self.sync_audio_monitor("timeline-pointer-commit");
        self.adopt_locus_label();
        self.last_inflight_key = None;
        self.log_semantic_state("timeline-pointer-commit", None);
        self.preview_dirty = true;
    }

    fn cancel_timeline_pointer(&mut self) {
        let _ = self.session.cancel_timeline_pointer();
        trace::log("gesture cancel");
        self.preview_dirty = true;
    }

    fn apply_rail_width(&mut self) {
        let width = self
            .rail_geom
            .lock()
            .map(|g| f64::from(g.1))
            .unwrap_or(f64::from(TIMELINE_WIDTH));
        self.session.set_rail_width(width.max(1.0));
    }

    fn canvas_point(
        &self,
        position: gpui::Point<gpui::Pixels>,
        canvas: CanvasSize,
    ) -> Option<CanvasPoint> {
        let (origin_x, origin_y, width, height) =
            self.canvas_geom.lock().ok()?.as_ref().copied()?;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        Some(CanvasPoint::new(
            f64::from(f32::from(position.x) - origin_x) / f64::from(width) * canvas.width,
            f64::from(f32::from(position.y) - origin_y) / f64::from(height) * canvas.height,
        ))
    }

    fn begin_canvas_pointer(
        &mut self,
        locus_id: &str,
        overlay: CanvasRect,
        canvas: CanvasSize,
        position: gpui::Point<gpui::Pixels>,
    ) {
        let Some(pointer) = self.canvas_point(position, canvas) else {
            trace::log("canvas drag begin: frame geometry unavailable");
            return;
        };
        let geometry = self.canvas_geom.lock().ok().and_then(|slot| *slot);
        trace::log(format!(
            "canvas drag geometry={geometry:?} pointer={pointer:?} overlay={overlay:?} canvas={canvas:?}"
        ));
        self.play_origin = None;
        self.audio_play_pending = false;
        self.preview_inbox.clear_pending();
        match self
            .session
            .begin_canvas_overlay_drag(locus_id, overlay, canvas, pointer)
        {
            Ok(()) => {
                self.adopt_locus_label();
                trace::log(format!(
                    "canvas drag begin locus={locus_id} pointer={:.1},{:.1}",
                    pointer.x, pointer.y
                ));
                self.last_inflight_key = None;
                self.log_semantic_state("canvas-drag-begin", None);
            }
            Err(err) => {
                trace::log(format!("canvas drag begin: {err}"));
                self.last_render = Some(format!("canvas drag: {err}"));
            }
        }
        self.sync_audio_monitor("canvas-drag");
    }

    fn begin_canvas_resize_pointer(
        &mut self,
        locus_id: &str,
        corner: ResizeCorner,
        overlay: CanvasRect,
        canvas: CanvasSize,
        position: gpui::Point<gpui::Pixels>,
    ) {
        let Some(pointer) = self.canvas_point(position, canvas) else {
            trace::log("canvas resize begin: frame geometry unavailable");
            return;
        };
        self.play_origin = None;
        self.audio_play_pending = false;
        self.preview_inbox.clear_pending();
        match self
            .session
            .begin_canvas_overlay_resize(locus_id, corner, overlay, canvas, pointer)
        {
            Ok(()) => {
                self.canvas_resize_corner = Some(corner);
                self.adopt_locus_label();
                trace::log(format!(
                    "canvas resize begin locus={locus_id} corner={corner:?} pointer={:.1},{:.1}",
                    pointer.x, pointer.y
                ));
                self.last_inflight_key = None;
                self.log_semantic_state("canvas-resize-begin", None);
            }
            Err(err) => {
                trace::log(format!("canvas resize begin: {err}"));
                self.last_render = Some(format!("canvas resize: {err}"));
            }
        }
        self.sync_audio_monitor("canvas-resize");
    }

    fn update_canvas_pointer(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        canvas: CanvasSize,
    ) -> bool {
        if self.session.canvas_overlay_resize_active() {
            if let Some(pointer) = self.canvas_point(position, canvas)
                && let Err(err) = self.session.update_canvas_overlay_resize(pointer)
            {
                trace::log(format!("canvas resize update: {err}"));
            }
            self.log_inflight_semantic_state("canvas-resize-update");
            return true;
        }
        if !self.session.canvas_overlay_drag_active() {
            return false;
        }
        if let Some(pointer) = self.canvas_point(position, canvas)
            && let Err(err) = self.session.update_canvas_overlay_drag(pointer)
        {
            trace::log(format!("canvas drag update: {err}"));
        }
        self.log_inflight_semantic_state("canvas-drag-update");
        true
    }

    fn commit_canvas_pointer(
        &mut self,
        position: gpui::Point<gpui::Pixels>,
        canvas: CanvasSize,
    ) -> bool {
        if self.session.canvas_overlay_resize_active() {
            let outcome = match self.canvas_point(position, canvas) {
                None => {
                    self.session.cancel_canvas_overlay_resize();
                    Err("frame geometry unavailable".to_string())
                }
                Some(pointer) => self
                    .session
                    .commit_canvas_overlay_resize(pointer)
                    .map_err(|err| err.to_string()),
            };
            self.canvas_resize_corner = None;
            match outcome {
                Ok(outcome) => trace::log(format!(
                    "canvas resize commit outcome={outcome:?} undo={}",
                    self.session.undo_len()
                )),
                Err(err) => {
                    trace::log(format!("canvas resize commit: {err}"));
                    self.last_render = Some(format!("canvas resize: {err}"));
                }
            }
            self.adopt_locus_label();
            self.last_inflight_key = None;
            self.log_semantic_state("canvas-resize-commit", None);
            self.refresh_preview("canvas-resize");
            self.queue_preview();
            return true;
        }
        if !self.session.canvas_overlay_drag_active() {
            return false;
        }
        let outcome = match self.canvas_point(position, canvas) {
            None => {
                self.session.cancel_canvas_overlay_drag();
                Err("frame geometry unavailable".to_string())
            }
            Some(pointer) => self
                .session
                .commit_canvas_overlay_drag(pointer)
                .map_err(|err| err.to_string()),
        };
        match outcome {
            Ok(outcome) => trace::log(format!(
                "canvas drag commit outcome={outcome:?} undo={}",
                self.session.undo_len()
            )),
            Err(err) => {
                trace::log(format!("canvas drag commit: {err}"));
                self.last_render = Some(format!("canvas drag: {err}"));
            }
        }
        self.adopt_locus_label();
        self.last_inflight_key = None;
        self.log_semantic_state("canvas-drag-commit", None);
        self.refresh_preview("canvas-drag");
        self.queue_preview();
        true
    }

    fn cancel_canvas_pointer(&mut self) -> bool {
        if self.session.canvas_overlay_resize_active() {
            let outcome = self.session.cancel_canvas_overlay_resize();
            self.canvas_resize_corner = None;
            trace::log(format!("canvas resize cancel outcome={outcome:?}"));
            self.refresh_preview("canvas-resize-cancel");
            self.queue_preview();
            return true;
        }
        if !self.session.canvas_overlay_drag_active() {
            return false;
        }
        let outcome = self.session.cancel_canvas_overlay_drag();
        trace::log(format!("canvas drag cancel outcome={outcome:?}"));
        self.refresh_preview("canvas-drag-cancel");
        self.queue_preview();
        true
    }

    fn drain_preview(&mut self) -> bool {
        let taken = self
            .preview_slot
            .lock()
            .ok()
            .and_then(|mut slot| slot.take());
        let Some((generation, result, time, stamp)) = taken else {
            return false;
        };
        match result {
            Ok((frame, selection)) => {
                let shown = format!("memory {}x{}", frame.width, frame.height);
                let frame = Arc::new(frame);
                if self.session.accept_preview_frame_stamped(
                    generation,
                    Arc::clone(&frame),
                    time,
                    &stamp,
                ) {
                    match render_image_from_raw(&frame) {
                        Ok(image) => {
                            self.preview_previous = self.preview_current.replace(image);
                            self.renderer_selection = Some(selection);
                            self.renderer_error = None;
                            self.preview_retry_required = false;
                        }
                        Err(err) => {
                            trace::log(format!("preview image conversion err {err}"));
                            return false;
                        }
                    }
                    self.preview_shown = true;
                    trace::log(format!("preview frame {} {shown}", format_time(time)));
                    return true;
                }
            }
            Err(err) => {
                trace::log(format!("preview worker err {err}"));
                let mailbox = self.session.preview_mailbox();
                if generation == mailbox.current_generation() && stamp == mailbox.stamp() {
                    self.block_play_on_preview_error(err);
                    return true;
                }
            }
        }
        false
    }

    fn block_play_on_preview_error(&mut self, message: String) {
        self.audio_play_pending = false;
        self.session.pause();
        self.play_origin = None;
        self.sync_audio_monitor("preview-renderer-error");
        self.preview_inbox.clear_pending();
        self.preview_dirty = false;
        self.preview_retry_required = true;
        self.renderer_error = Some(message.clone());
        trace::log(format!(
            "preview renderer blocks A/V play requested={} error={message}",
            self.renderer
        ));
    }

    fn start_play(&mut self) {
        if self.preview_retry_required {
            self.audio_play_pending = false;
            self.session.pause();
            self.play_origin = None;
            self.sync_audio_monitor("preview-retry-required");
            self.preview_inbox.clear_pending();
            trace::log("play blocked: explicitly select a renderer to retry preview");
            return;
        }
        let stamp = self.session.audio_prepare_stamp();
        if self.audio_program_stamp.as_deref() != Some(stamp.as_str()) && !self.audio_preparing {
            self.audio_error = None;
            self.queue_audio_prepare();
        }
        let ready = self.audio_no_windows
            || (self.audio_monitor.is_some()
                && self.audio_program_stamp.as_deref() == Some(stamp.as_str()));
        if ready {
            self.begin_ready_play();
            return;
        }
        if self.audio_error.is_some() {
            self.audio_play_pending = false;
            self.session.pause();
            self.play_origin = None;
            trace::log(format!("play blocked: {}", self.audio_status()));
            return;
        }
        self.audio_play_pending = true;
        self.session.pause();
        self.play_origin = None;
        self.preview_inbox.clear_pending();
        self.refresh_preview("audio-play-pending");
        self.queue_preview();
        trace::log(format!("play pending: {}", self.audio_status()));
    }

    fn queue_preview(&mut self) {
        self.last_preview_time = None;
        self.queue_preview_if_needed();
    }

    fn queue_preview_if_needed(&mut self) {
        if self.preview_retry_required || !preview_extract_enabled() {
            return;
        }
        let time = self.session.snapped_preview_time();
        if !self.preview_dirty && self.last_preview_time == Some(time) {
            return;
        }
        self.preview_dirty = false;
        self.last_preview_time = Some(time);
        let job = self
            .session
            .request_preview_job_with_renderer(self.renderer);
        if self.preview_sampler_reset_required {
            self.preview_inbox.request_sampler_reset(job.generation);
            self.preview_sampler_reset_required = false;
        }
        self.preview_inbox.push(job);
    }

    fn set_renderer(&mut self, renderer: RendererRequest) {
        if self.renderer == renderer && !self.preview_retry_required {
            return;
        }
        self.catch_up_play_clock();
        self.session.pause();
        self.play_origin = None;
        self.audio_play_pending = false;
        self.sync_audio_monitor("renderer-change");
        self.preview_inbox.clear_pending();
        self.renderer = renderer;
        self.renderer_selection = None;
        self.renderer_error = None;
        self.preview_retry_required = false;
        self.preview_sampler_reset_required = true;
        self.refresh_preview("renderer-change");
        self.queue_preview();
        trace::log(format!("renderer requested={renderer}"));
    }

    fn renderer_status(&self) -> String {
        if let Some(error) = &self.renderer_error {
            return format!("{} error: {error}", renderer_short(self.renderer));
        }
        self.renderer_selection.as_ref().map_or_else(
            || format!("{} initializing", renderer_short(self.renderer)),
            ToString::to_string,
        )
    }
}

fn preview_worker_loop(inbox: std::sync::Arc<PreviewInbox>, slot: Arc<Mutex<PreviewSlot>>) {
    let engine = Engine::default();
    let mut sampler: Option<PreviewSamplerCache> = None;
    while let Some(job) = inbox.take_wait() {
        if inbox.take_sampler_reset(job.generation) {
            sampler = None;
            trace::log("preview renderer sampler reset by explicit selection");
        }
        let stamp = job.stamp.clone();
        let generation = job.generation;
        let time = job.timeline_time;
        let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            preview_worker_sample(&engine, &mut sampler, job)
        })) {
            Ok(result) => result,
            Err(payload) => {
                sampler = None;
                let msg = payload
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "preview worker panic".into());
                trace::log(format!("preview worker panic: {msg}"));
                Err(msg)
            }
        };
        inbox.complete(generation);
        if let Ok(mut guard) = slot.lock() {
            *guard = Some((generation, result, time, stamp));
        }
    }
}

fn audio_prepare_worker_loop(inbox: Arc<AudioPrepareInbox>, slot: Arc<Mutex<AudioPrepareSlot>>) {
    while let Some(request) = inbox.take_wait() {
        let generation = request.generation;
        let stamp = request.stamp;
        let format = request.format;
        let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            AudioProgram::prepare_job(&request.job, format).map_err(|err| err.to_string())
        })) {
            Ok(result) => result,
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .map(|message| (*message).to_string())
                    .or_else(|| payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "audio prepare worker panic".into());
                Err(message)
            }
        };
        if let Ok(mut slot) = slot.lock() {
            *slot = Some((generation, stamp, result));
        }
    }
}

struct PreviewSamplerCache {
    stamp: String,
    lock_stamp: String,
    width: u32,
    height: u32,
    fps_num: i64,
    fps_den: i64,
    renderer: RendererRequest,
    sampler: PreviewSampler,
}

impl PreviewSamplerCache {
    fn exactly_matches(&self, job: &lattice_studio::PreviewJob) -> bool {
        self.stamp == job.stamp
            && self.lock_stamp == job.lock_stamp
            && self.width == job.width
            && self.height == job.height
            && self.fps_num == job.fps_num
            && self.fps_den == job.fps_den
            && self.renderer == job.renderer
    }

    fn update_key(&mut self, job: &lattice_studio::PreviewJob) {
        self.stamp.clone_from(&job.stamp);
        self.lock_stamp.clone_from(&job.lock_stamp);
        self.width = job.width;
        self.height = job.height;
        self.fps_num = job.fps_num;
        self.fps_den = job.fps_den;
        self.renderer = job.renderer;
    }
}

fn preview_worker_sample(
    engine: &Engine,
    sampler: &mut Option<PreviewSamplerCache>,
    job: lattice_studio::PreviewJob,
) -> PreviewWorkerResult {
    let key_ok = sampler
        .as_ref()
        .is_some_and(|cached| cached.exactly_matches(&job));
    if !key_ok {
        let request = PreviewFrameRequest {
            timeline_time: job.timeline_time,
            width: job.width,
            height: job.height,
            fps_num: job.fps_num,
            fps_den: job.fps_den,
        };
        let output_hint = job.media_root.join(".lattice-cache").join("studio-sample");
        let spec = OutputSpec {
            width: job.width,
            height: job.height,
            fps_num: job.fps_num,
            fps_den: job.fps_den,
            sample_rate: 44_100,
            channels: 2,
        };
        let mut options = PreviewOptions::new(output_hint.clone(), job.media_root.clone());
        options.lock = Engine::load_lock(&job.media_root);
        options.renderer = job.renderer;
        let rebound = if let Some(cached) = sampler.as_mut() {
            let timeline =
                Engine::timeline(&job.compilation.project).map_err(|err| err.to_string())?;
            match cached.sampler.rebind_timeline(timeline, spec, &options) {
                Ok(()) => {
                    cached.update_key(&job);
                    trace::log(format!(
                        "preview renderer rebound {}",
                        cached.sampler.selection()
                    ));
                    true
                }
                Err(err) => {
                    trace::log(format!("preview renderer recreate: {err}"));
                    false
                }
            }
        } else {
            false
        };
        if !rebound {
            let next = engine
                .sample_session(
                    &job.compilation.project,
                    &request,
                    &job.media_root,
                    &output_hint,
                    options.lock.as_ref(),
                    job.renderer,
                )
                .map_err(|err| err.to_string())?;
            trace::log(format!("preview renderer {}", next.selection()));
            *sampler = Some(PreviewSamplerCache {
                stamp: job.stamp.clone(),
                lock_stamp: job.lock_stamp.clone(),
                width: job.width,
                height: job.height,
                fps_num: job.fps_num,
                fps_den: job.fps_den,
                renderer: job.renderer,
                sampler: next,
            });
        }
    }
    sampler.as_mut().map_or_else(
        || Err("preview sampler missing".into()),
        |cached| {
            let selection = cached.sampler.selection().clone();
            cached
                .sampler
                .sample(job.timeline_time)
                .map(|(_, frame)| (frame, selection))
                .map_err(|err| err.to_string())
        },
    )
}

/// GPUI consumes BGRA frames. Keep conversion at the Studio boundary so Core/Engine continue to
/// expose backend-neutral RGBA `RawFrame` values.
fn render_image_from_raw(frame: &RawFrame) -> Result<Arc<RenderImage>, String> {
    let expected = usize::try_from(frame.width)
        .ok()
        .and_then(|width| {
            usize::try_from(frame.height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "preview dimensions overflow".to_string())?;
    if frame.rgba.len() != expected {
        return Err(format!(
            "preview byte length {} != expected {expected}",
            frame.rgba.len()
        ));
    }
    let mut bgra = frame.rgba.clone();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    let image = image::RgbaImage::from_raw(frame.width, frame.height, bgra)
        .ok_or_else(|| "invalid preview image buffer".to_string())?;
    let frame = image::Frame::from_parts(image, 0, 0, image::Delay::from_numer_denom_ms(0, 1));
    Ok(Arc::new(RenderImage::new([frame])))
}

impl Drop for StudioView {
    fn drop(&mut self) {
        self.preview_inbox.stop();
        self.audio_inbox.stop();
        if let Some(monitor) = self.audio_monitor.as_mut() {
            let _ = monitor.pause(self.session.playhead());
        }
        trace::log("StudioView dropped");
    }
}

impl Focusable for StudioView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

#[cfg(feature = "window")]
const BG: u32 = 0x0c0e12;
#[cfg(feature = "window")]
const PANEL: u32 = 0x141821;
#[cfg(feature = "window")]
const LINE: u32 = 0x2a3140;
#[cfg(feature = "window")]
const TEXT: u32 = 0xe8edf5;
#[cfg(feature = "window")]
const MUTED: u32 = 0x8b95a8;
#[cfg(feature = "window")]
const TEAL: u32 = 0x3dd6c6;

#[cfg(feature = "window")]
impl Render for StudioView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = match self.session.layout() {
            Ok(layout) => Some(layout),
            Err(err) => {
                trace::log(format!("layout failed: {err}"));
                None
            }
        };
        if !self.first_paint_logged {
            self.first_paint_logged = true;
            let preview = layout
                .as_ref()
                .and_then(|item| item.canvas.preview_frame.as_ref())
                .map_or_else(|| "none".into(), |path| path.display().to_string());
            trace::log(format!("first paint preview={preview}"));
            self.log_semantic_state("first-paint", Some(window));
        }
        self.refresh_focus(window);
        self.maybe_log_smoke_geom();
        let file = layout
            .as_ref()
            .map(|item| item.file_label.clone())
            .unwrap_or_else(|| "main.vel".into());
        let cursor = if self.session.canvas_overlay_resize_active() {
            self.canvas_resize_corner
                .map_or(CursorStyle::Arrow, resize_cursor)
        } else if self.session.canvas_overlay_drag_active() {
            CursorStyle::ClosedHand
        } else {
            self.hover_x.map_or(CursorStyle::Arrow, |x| {
                cursor_style(self.session.cursor_at_on(x, self.hover_track.as_deref()))
            })
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .text_sm()
            .track_focus(&self.focus)
            .cursor(cursor)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_key(event, cx);
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                let (width, height) = this.session.preview_pixel_size();
                if this.update_canvas_pointer(
                    event.position,
                    CanvasSize::new(f64::from(width), f64::from(height)),
                ) {
                    cx.notify();
                    return;
                }
                let x = this.pointer_x(event.position.x);
                let track = this.track_at(event.position);
                let cursor = this.session.cursor_at_on(x, track.as_deref());
                let prev_cursor = this.hover_x.map_or(CursorKind::Select, |prev| {
                    this.session.cursor_at_on(prev, this.hover_track.as_deref())
                });
                let hover_changed = cursor != prev_cursor || this.hover_track != track;
                this.hover_x = Some(x);
                this.hover_track = track;
                if this.session.gesture().is_active() {
                    this.update_timeline_pointer(event.position.x, event.modifiers.alt);
                    this.queue_preview_if_needed();
                    cx.notify();
                } else if hover_changed {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    let (width, height) = this.session.preview_pixel_size();
                    if this.commit_canvas_pointer(
                        event.position,
                        CanvasSize::new(f64::from(width), f64::from(height)),
                    ) {
                        cx.notify();
                        return;
                    }
                    let active = this.session.gesture().is_active();
                    this.commit_timeline_pointer(event.position.x, event.modifiers.alt);
                    if active {
                        this.queue_preview();
                        cx.notify();
                    }
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    let (width, height) = this.session.preview_pixel_size();
                    if this.commit_canvas_pointer(
                        event.position,
                        CanvasSize::new(f64::from(width), f64::from(height)),
                    ) {
                        cx.notify();
                        return;
                    }
                    let active = this.session.gesture().is_active();
                    this.commit_timeline_pointer(event.position.x, event.modifiers.alt);
                    if active {
                        this.queue_preview();
                        cx.notify();
                    }
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                this.handle_scroll(event, cx);
            }))
            .child(header_bar(&file))
            .child(self.actions_bar(cx))
            .child(self.body(layout.as_ref(), cx))
            .child(self.timeline_bar(layout.as_ref(), cx))
    }
}

fn cursor_style(kind: CursorKind) -> CursorStyle {
    match kind {
        CursorKind::Trim => CursorStyle::ResizeLeftRight,
        CursorKind::Grab => CursorStyle::OpenHand,
        CursorKind::Grabbing => CursorStyle::ClosedHand,
        CursorKind::Scrub => CursorStyle::IBeam,
        CursorKind::Select => CursorStyle::Arrow,
    }
}

fn resize_cursor(corner: ResizeCorner) -> CursorStyle {
    match corner {
        ResizeCorner::TopLeft | ResizeCorner::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeCorner::TopRight | ResizeCorner::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}

fn canvas_resize_handle(
    locus_id: String,
    corner: ResizeCorner,
    active_corner: Option<ResizeCorner>,
    overlay: CanvasRect,
    canvas: CanvasSize,
    cx: &mut Context<StudioView>,
) -> impl IntoElement {
    const HANDLE: f32 = 12.0;
    const OFFSET: f32 = -6.0;
    let corner_name = match corner {
        ResizeCorner::TopLeft => "top-left",
        ResizeCorner::TopRight => "top-right",
        ResizeCorner::BottomLeft => "bottom-left",
        ResizeCorner::BottomRight => "bottom-right",
    };
    let mut handle = div()
        .id(SharedString::from(format!(
            "canvas-resize-{locus_id}-{corner_name}"
        )))
        .debug_selector({
            let locus_id = locus_id.clone();
            move || format!("canvas.resize.{locus_id}.{corner_name}")
        })
        .absolute()
        .w(px(HANDLE))
        .h(px(HANDLE))
        .bg(rgb(TEAL))
        .border_1()
        .border_color(rgb(BG))
        .cursor(resize_cursor(active_corner.unwrap_or(corner)))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                this.begin_canvas_resize_pointer(
                    &locus_id,
                    corner,
                    overlay,
                    canvas,
                    event.position,
                );
                cx.stop_propagation();
                cx.notify();
            }),
        );
    handle = match corner {
        ResizeCorner::TopLeft => handle.left(px(OFFSET)).top(px(OFFSET)),
        ResizeCorner::TopRight => handle.right(px(OFFSET)).top(px(OFFSET)),
        ResizeCorner::BottomLeft => handle.left(px(OFFSET)).bottom(px(OFFSET)),
        ResizeCorner::BottomRight => handle.right(px(OFFSET)).bottom(px(OFFSET)),
    };
    handle
}

#[cfg(feature = "window")]
impl StudioView {
    fn actions_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_1()
            .px_2()
            .py_1()
            .border_b_1()
            .border_color(rgb(LINE))
            .bg(rgb(PANEL))
            .child(action_button("Open Video…", LINE, cx, move |this, cx| {
                this.open_video_clicked();
                cx.notify();
            }))
            .child(action_button("Set In", LINE, cx, move |this, cx| {
                if let Err(err) = this.session.set_in_at_playhead() {
                    trace::log(format!("set in: {err}"));
                }
                this.after_edit();
                cx.notify();
            }))
            .child(action_button("Set Out", LINE, cx, move |this, cx| {
                if let Err(err) = this.session.set_out_at_playhead() {
                    trace::log(format!("set out: {err}"));
                }
                this.after_edit();
                cx.notify();
            }))
            .child(action_button(
                "Split at Playhead",
                LINE,
                cx,
                move |this, cx| {
                    if let Err(err) = this.session.split_at_playhead() {
                        trace::log(format!("split: {err}"));
                    }
                    this.after_edit();
                    cx.notify();
                },
            ))
            .child(action_button(
                "Delete Selected Clip",
                LINE,
                cx,
                move |this, cx| {
                    if let Err(err) = this.session.delete_selected_clip() {
                        trace::log(format!("delete clip: {err}"));
                    }
                    this.after_edit();
                    cx.notify();
                },
            ))
            .child(
                div()
                    .debug_selector(|| "toolbar.renderer-status".into())
                    .px_2()
                    .text_color(if self.renderer_error.is_some() {
                        rgb(0xff8f8f)
                    } else {
                        rgb(MUTED)
                    })
                    .child(format!("Renderer · {}", self.renderer_status())),
            )
            .child(
                div()
                    .debug_selector(|| "toolbar.audio-status".into())
                    .px_2()
                    .text_color(if self.audio_error.is_some() {
                        rgb(0xff8f8f)
                    } else {
                        rgb(MUTED)
                    })
                    .child(self.audio_status()),
            )
            .child(action_button(
                "CPU",
                if self.renderer == RendererRequest::RequireCpu {
                    TEAL
                } else {
                    LINE
                },
                cx,
                move |this, cx| {
                    this.set_renderer(RendererRequest::RequireCpu);
                    cx.notify();
                },
            ))
            .child(action_button(
                "GPU DX12",
                if self.renderer == RendererRequest::RequireGpuDx12 {
                    TEAL
                } else {
                    LINE
                },
                cx,
                move |this, cx| {
                    this.set_renderer(RendererRequest::RequireGpuDx12);
                    cx.notify();
                },
            ))
            .child({
                let geom = Arc::clone(&self.play_geom);
                div()
                    .id("Play")
                    .debug_selector(|| "toolbar.play".into())
                    .relative()
                    .px_3()
                    .py_1()
                    .bg(rgb(TEAL))
                    .text_color(rgb(TEXT))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.start_play();
                        cx.notify();
                    }))
                    .child("Play")
                    .child(
                        canvas(
                            move |bounds, _, _| {
                                if let Ok(mut slot) = geom.lock() {
                                    *slot = Some((
                                        f32::from(bounds.origin.x),
                                        f32::from(bounds.origin.y),
                                        f32::from(bounds.size.width),
                                        f32::from(bounds.size.height),
                                    ));
                                }
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
            })
            .child(action_button("Pause", LINE, cx, move |this, cx| {
                this.catch_up_play_clock();
                this.audio_play_pending = false;
                this.session.pause();
                this.play_origin = None;
                this.sync_audio_monitor("pause");
                this.preview_inbox.clear_pending();
                this.refresh_preview("pause");
                this.queue_preview();
                trace::log("pause");
                cx.notify();
            }))
            .child(action_button("Seek", LINE, cx, move |this, cx| {
                this.audio_play_pending = false;
                this.session.seek(lattice_engine::Time::ZERO);
                this.play_origin = None;
                this.sync_audio_monitor("seek");
                this.preview_inbox.clear_pending();
                this.refresh_preview("seek");
                this.queue_preview();
                cx.notify();
            }))
            .child(action_button("Scrub", LINE, cx, move |this, cx| {
                this.audio_play_pending = false;
                this.session.scrub(this.session.playhead());
                this.play_origin = None;
                this.sync_audio_monitor("scrub");
                this.preview_inbox.clear_pending();
                this.refresh_preview("scrub");
                this.queue_preview();
                cx.notify();
            }))
            .child(action_button("Save", TEAL, cx, move |this, cx| {
                let _ = this.session.save();
                cx.notify();
            }))
            .child(action_button("Undo", LINE, cx, move |this, cx| {
                if let Err(err) = this.session.undo() {
                    trace::log(format!("undo: {err}"));
                }
                this.after_edit();
                cx.notify();
            }))
            .child(action_button("Redo", LINE, cx, move |this, cx| {
                if let Err(err) = this.session.redo() {
                    trace::log(format!("redo: {err}"));
                }
                this.after_edit();
                cx.notify();
            }))
            .child(action_button("Resolve", TEAL, cx, move |this, cx| {
                match this.session.resolve_media() {
                    Ok(resolution) => {
                        this.last_render = Some(format!(
                            "resolved {} assets ({} provider calls)",
                            resolution.assets.len(),
                            resolution.provider_calls
                        ));
                        this.invalidate_audio("resolve");
                        this.refresh_preview("resolve");
                        this.queue_preview();
                    }
                    Err(err) => {
                        trace::log(format!("resolve: {err}"));
                        this.last_render = Some(format!("resolve failed: {err}"));
                    }
                }
                cx.notify();
            }))
            .child(action_button(
                "Copy locus JSON",
                LINE,
                cx,
                move |this, cx| {
                    match this.session.current_projection_json() {
                        Ok(Some(json)) => {
                            cx.write_to_clipboard(ClipboardItem::new_string(json));
                            this.last_render = Some("copied locus JSON".into());
                        }
                        Ok(None) => {
                            this.last_render = Some("no current locus".into());
                        }
                        Err(err) => {
                            trace::log(format!("copy locus: {err}"));
                            this.last_render = Some(format!("copy locus failed: {err}"));
                        }
                    }
                    cx.notify();
                },
            ))
            .child(action_button("Gain -3 dB", LINE, cx, move |this, cx| {
                let _ = this.session.set_gain(-3);
                this.after_edit();
                cx.notify();
            }))
            .child(action_button("Fade", LINE, cx, move |this, cx| {
                let _ = this
                    .session
                    .set_fade(lattice_engine::Time::milliseconds(500));
                this.after_edit();
                cx.notify();
            }))
            .child(action_button("Zoom In", LINE, cx, move |this, cx| {
                this.session.zoom_around(this.session.playhead(), 1.25);
                cx.notify();
            }))
            .child(action_button("Zoom Out", LINE, cx, move |this, cx| {
                this.session.zoom_around(this.session.playhead(), 0.8);
                cx.notify();
            }))
    }

    fn open_video_clicked(&mut self) {
        let Some(path) = open_video_path() else {
            self.last_render =
                Some("Open Video…: set LATTICE_OPEN_VIDEO to an MP4 path, or pick a file".into());
            return;
        };
        match StudioSession::open_video(&path) {
            Ok(session) => {
                trace::log(format!("open_video ok {}", path.display()));
                self.session = session;
                self.adopt_locus_label();
                self.last_render = Some(format!("opened {}", path.display()));
                if let Ok(mut slot) = self.preview_slot.lock() {
                    *slot = None;
                }
                self.after_edit();
            }
            Err(err) => {
                trace::log(format!("open_video failed: {err}"));
                self.last_render = Some(format!("open video: {err}"));
            }
        }
    }

    fn refresh_preview(&mut self, why: &str) {
        trace::log(format!("preview queue {why}"));
        self.preview_dirty = true;
    }

    fn after_edit(&mut self) {
        self.sync_source_draft();
        self.invalidate_audio("edit");
        self.preview_dirty = true;
        self.queue_preview();
    }

    fn track_at(&self, position: gpui::Point<gpui::Pixels>) -> Option<String> {
        let x = f32::from(position.x);
        let y = f32::from(position.y);
        let geoms = self.track_geoms.lock().ok()?;
        geoms.iter().rev().find_map(|(name, gx, gy, gw, gh)| {
            (x >= *gx && x <= *gx + *gw && y >= *gy && y <= *gy + *gh).then(|| name.clone())
        })
    }

    fn handle_source_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        if event.keystroke.modifiers.control && key == "a" {
            let end = self.source_draft.encode_utf16().count();
            self.source_selection_utf16 = 0..end;
            self.source_marked_utf16 = None;
            cx.stop_propagation();
            cx.notify();
            return;
        }

        let selection = clamp_utf16_range(&self.source_draft, self.source_selection_utf16.clone());
        let mut replacement = None;
        match key {
            "backspace" => {
                let range = if selection.is_empty() {
                    previous_utf16_boundary(&self.source_draft, selection.start)..selection.end
                } else {
                    selection
                };
                replacement = Some((range, ""));
            }
            "delete" => {
                let range = if selection.is_empty() {
                    selection.start..next_utf16_boundary(&self.source_draft, selection.end)
                } else {
                    selection
                };
                replacement = Some((range, ""));
            }
            "enter" => replacement = Some((selection, "\n")),
            "tab" => replacement = Some((selection, "  ")),
            "left" => {
                let caret = if selection.is_empty() {
                    previous_utf16_boundary(&self.source_draft, selection.start)
                } else {
                    selection.start
                };
                self.source_selection_utf16 = caret..caret;
            }
            "right" => {
                let caret = if selection.is_empty() {
                    next_utf16_boundary(&self.source_draft, selection.end)
                } else {
                    selection.end
                };
                self.source_selection_utf16 = caret..caret;
            }
            "home" => self.source_selection_utf16 = 0..0,
            "end" => {
                let end = self.source_draft.encode_utf16().count();
                self.source_selection_utf16 = end..end;
            }
            _ => return,
        }

        if let Some((range, text)) = replacement {
            self.replace_source_range(range, text, None);
            self.commit_source_draft();
        }
        self.source_marked_utf16 = None;
        cx.stop_propagation();
        cx.notify();
    }

    fn handle_key(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if event.keystroke.modifiers.control && event.keystroke.key == "z" {
            let result = if event.keystroke.modifiers.shift {
                self.session.redo()
            } else {
                self.session.undo()
            };
            if let Err(err) = result {
                trace::log(format!("undo/redo: {err}"));
            }
            self.after_edit();
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if event.keystroke.modifiers.control && event.keystroke.key == "y" {
            if let Err(err) = self.session.redo() {
                trace::log(format!("redo: {err}"));
            }
            self.after_edit();
            cx.stop_propagation();
            cx.notify();
            return;
        }
        match event.keystroke.key.as_str() {
            "escape" => {
                if !self.cancel_canvas_pointer() {
                    self.cancel_timeline_pointer();
                }
                cx.notify();
            }
            "=" | "+" => {
                self.session.zoom_around(self.session.playhead(), 1.25);
                cx.notify();
            }
            "-" => {
                self.session.zoom_around(self.session.playhead(), 0.8);
                cx.notify();
            }
            _ => {}
        }
    }

    fn handle_scroll(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let x = self.pointer_x(event.position.x);
        self.apply_rail_width();
        let delta = match event.delta {
            ScrollDelta::Pixels(pt) => f64::from(f32::from(pt.x) + f32::from(pt.y)),
            ScrollDelta::Lines(pt) => f64::from(f32::from(pt.x) + f32::from(pt.y)) * 40.0,
        };
        if event.modifiers.control {
            let factor = if delta < 0.0 { 1.15 } else { 1.0 / 1.15 };
            let anchor = self.session.time_at_x(x);
            self.session.zoom_around(anchor, factor);
        } else {
            self.session.scroll_pixels(delta);
        }
        cx.notify();
    }

    fn body(
        &self,
        layout: Option<&lattice_studio::StudioLayout>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let Some(layout) = layout else {
            return div().flex_1().child("layout failed");
        };
        div()
            .flex()
            .flex_row()
            .flex_1()
            .min_h(px(0.0))
            .child(self.tree_pane(layout, cx))
            .child(self.canvas_pane(layout, cx))
            .child(self.source_pane(layout, cx))
            .child(self.inspector_pane(layout, cx))
    }

    fn tree_pane(
        &self,
        layout: &lattice_studio::StudioLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        pane("SEQUENCE", px(200.0), tree_nodes(&layout.tree, cx))
    }

    fn canvas_pane(
        &self,
        layout: &lattice_studio::StudioLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let overlays = layout.canvas.overlays.clone();
        let active_resize_corner = if self.session.canvas_overlay_resize_active() {
            self.canvas_resize_corner
        } else {
            None
        };
        if let Ok(mut geom) = self.canvas_geom.lock() {
            *geom = None;
        }
        let mut stage = div()
            .flex()
            .flex_col()
            .flex_1()
            .m_3()
            .bg(rgb(0x1a2330))
            .border_1()
            .border_color(rgb(LINE))
            .relative();
        let preview_image = self.preview_current.clone().map(img).or_else(|| {
            layout.canvas.preview_frame.clone().and_then(|path| {
                if path.is_file() {
                    Some(img(path))
                } else {
                    trace::log(format!("preview path missing {}", path.display()));
                    None
                }
            })
        });
        if let Some(preview_image) = preview_image {
            let width = layout.canvas.preview_width as f32;
            let height = layout.canvas.preview_height as f32;
            let canvas_size = CanvasSize::new(f64::from(width), f64::from(height));
            let geom = Arc::clone(&self.canvas_geom);
            let mut frame = div()
                .relative()
                .w(px(width.max(1.0)))
                .h(px(height.max(1.0)))
                .child(
                    preview_image
                        .object_fit(ObjectFit::Contain)
                        .id("canvas-frame")
                        .debug_selector(|| "canvas.frame".into())
                        .w(px(width.max(1.0)))
                        .h(px(height.max(1.0))),
                )
                .child(
                    canvas(
                        move |bounds, _, _| {
                            if let Ok(mut slot) = geom.lock() {
                                *slot = Some((
                                    f32::from(bounds.origin.x),
                                    f32::from(bounds.origin.y),
                                    f32::from(bounds.size.width),
                                    f32::from(bounds.size.height),
                                ));
                            }
                        },
                        |_, _, _, _| {},
                    )
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .size_full(),
                );
            for overlay in overlays {
                let id = overlay.locus_id.clone();
                let selected = overlay.selected;
                let left = overlay.x as f32;
                let top = overlay.y as f32;
                let ow = overlay.width.max(1) as f32;
                let oh = overlay.height.max(1) as f32;
                let drag_id = id.clone();
                let overlay_rect = CanvasRect::new(
                    f64::from(left),
                    f64::from(top),
                    f64::from(ow),
                    f64::from(oh),
                );
                let mut chrome = div()
                    .id(SharedString::from(format!("canvas-{id}")))
                    .debug_selector({
                        let id = id.clone();
                        move || format!("canvas.overlay.{id}")
                    })
                    .absolute()
                    .left(px(left))
                    .top(px(top))
                    .w(px(ow))
                    .h(px(oh))
                    .cursor(active_resize_corner.map_or_else(
                        || {
                            if selected && self.session.canvas_overlay_drag_active() {
                                CursorStyle::ClosedHand
                            } else {
                                CursorStyle::OpenHand
                            }
                        },
                        resize_cursor,
                    ))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                            this.begin_canvas_pointer(
                                &drag_id,
                                overlay_rect,
                                canvas_size,
                                event.position,
                            );
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    );
                if selected {
                    chrome = chrome
                        .border_2()
                        .border_color(rgb(TEAL))
                        .child(canvas_resize_handle(
                            id.clone(),
                            ResizeCorner::TopLeft,
                            active_resize_corner,
                            overlay_rect,
                            canvas_size,
                            cx,
                        ))
                        .child(canvas_resize_handle(
                            id.clone(),
                            ResizeCorner::TopRight,
                            active_resize_corner,
                            overlay_rect,
                            canvas_size,
                            cx,
                        ))
                        .child(canvas_resize_handle(
                            id.clone(),
                            ResizeCorner::BottomLeft,
                            active_resize_corner,
                            overlay_rect,
                            canvas_size,
                            cx,
                        ))
                        .child(canvas_resize_handle(
                            id.clone(),
                            ResizeCorner::BottomRight,
                            active_resize_corner,
                            overlay_rect,
                            canvas_size,
                            cx,
                        ));
                }
                frame = frame.child(chrome);
            }
            stage = stage.child(frame);
        } else {
            stage = stage.child(div().flex_1());
        }
        pane_flex("Canvas", stage)
    }

    fn source_pane(
        &self,
        layout: &lattice_studio::StudioLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let focus = self.source_focus.clone();
        let input_focus = focus.clone();
        let input_view = cx.entity();
        let scroll = self.source_scroll.clone();
        let selection =
            utf16_range_to_bytes(&self.source_draft, self.source_selection_utf16.clone());
        let locus_range = layout.source.highlight.map(|span| {
            usize::try_from(span.start).unwrap_or(0)
                ..usize::try_from(span.end).unwrap_or(self.source_draft.len())
        });
        let status = self.source_error.as_ref().map_or_else(
            || {
                layout.source.highlight.map_or_else(
                    || "click source to point the shared locus".to_string(),
                    |span| format!("locus at line {}", span.line),
                )
            },
            |error| format!("VEL: {error}"),
        );

        let mut editor = div()
            .id("vel-editor")
            .debug_selector(|| "vel.editor".into())
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.0))
            .overflow_scroll()
            .track_scroll(&scroll)
            .track_focus(&focus)
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_source_key(event, cx);
            }));

        for line in source_lines(&self.source_draft) {
            let line_range = line.start..line.end.saturating_add(1).min(self.source_draft.len());
            let selected = byte_ranges_intersect(selection.clone(), line_range.clone());
            let locus_highlighted = locus_range
                .as_ref()
                .is_some_and(|range| byte_ranges_intersect(range.clone(), line_range));
            let selector = format!("vel.line.{}", line.number);
            let line_number = line.number;
            let line_start = line.start;
            let line_text = line.text.clone();
            let line_scroll = scroll.clone();
            let line_focus = focus.clone();
            editor = editor.child(
                div()
                    .id(SharedString::from(format!("vel-line-{line_number}")))
                    .debug_selector(move || selector.clone().into())
                    .w_full()
                    .min_h(px(20.0))
                    .whitespace_nowrap()
                    .bg(if selected {
                        rgb(0x245a55)
                    } else if locus_highlighted {
                        rgb(0x1c3d3a)
                    } else {
                        rgb(PANEL)
                    })
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                            line_focus.focus(window);
                            let Some(bounds) = line_scroll.bounds_for_item(line_number - 1) else {
                                return;
                            };
                            let display = if line_text.is_empty() {
                                SharedString::from(" ")
                            } else {
                                SharedString::from(line_text.clone())
                            };
                            let style = window.text_style();
                            let run = TextRun {
                                len: display.len(),
                                font: style.font(),
                                color: style.color,
                                background_color: None,
                                underline: None,
                                strikethrough: None,
                            };
                            let font_size = style.font_size.to_pixels(window.rem_size());
                            let shaped =
                                window
                                    .text_system()
                                    .shape_line(display, font_size, &[run], None);
                            let local_x = px((f32::from(event.position.x)
                                - f32::from(bounds.left()))
                            .max(0.0));
                            let in_line = shaped.closest_index_for_x(local_x).min(line_text.len());
                            let offset = line_start.saturating_add(in_line);
                            let caret = byte_index_to_utf16(&this.source_draft, offset);
                            this.source_selection_utf16 = caret..caret;
                            this.source_marked_utf16 = None;
                            match u32::try_from(offset)
                                .ok()
                                .map(|offset| this.session.point_from_source_offset(offset))
                            {
                                Some(Ok(Some(_))) => this.adopt_locus_label(),
                                Some(Err(err)) => {
                                    this.source_error = Some(err.to_string());
                                }
                                _ => {}
                            }
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .child(if line.text.is_empty() {
                        " ".to_string()
                    } else {
                        line.text
                    }),
            );
        }

        div()
            .relative()
            .flex()
            .flex_col()
            .h_full()
            .w(px(280.0))
            .border_r_1()
            .border_color(rgb(LINE))
            .bg(rgb(PANEL))
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(LINE))
                    .text_color(rgb(TEAL))
                    .child("VEL"),
            )
            .child(
                div()
                    .id("vel-status")
                    .debug_selector(|| "vel.status".into())
                    .px_2()
                    .py_1()
                    .text_color(if self.source_error.is_some() {
                        rgb(0xff8a80)
                    } else {
                        rgb(MUTED)
                    })
                    .child(status),
            )
            .child(editor)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, (), window, cx| {
                        window.handle_input(
                            &input_focus,
                            StudioSourceInputHandler {
                                view: input_view,
                                bounds,
                            },
                            cx,
                        );
                    },
                )
                .absolute()
                .size_full(),
            )
    }

    fn inspector_pane(
        &self,
        layout: &lattice_studio::StudioLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let inspector = layout.inspector.clone();
        let review = layout.review.clone();
        let mut body = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(div().text_lg().child(inspector.heading.clone()))
            .child(div().text_color(rgb(MUTED)).child(inspector.origin.clone()))
            .child(
                div()
                    .text_color(rgb(TEAL))
                    .child(format!("Defined in {}", inspector.defined_in)),
            );
        if inspector.go_to_definition.is_some() {
            body = body.child(
                div()
                    .id("go-to-definition")
                    .debug_selector(|| "vel.go-to-definition".into())
                    .mt_2()
                    .px_3()
                    .py_1()
                    .bg(rgb(TEAL))
                    .text_color(rgb(BG))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        match this.session.go_to_definition() {
                            Ok(Some(span)) => {
                                let line = this.select_source_span(span);
                                this.source_scroll.scroll_to_top_of_item(line);
                                this.source_focus.focus(window);
                                this.last_render =
                                    Some(format!("VEL definition line {} selected", span.line));
                            }
                            Ok(None) => {
                                this.last_render = Some("current locus has no source span".into());
                            }
                            Err(err) => {
                                trace::log(format!("go to definition: {err}"));
                                this.last_render = Some(format!("go to definition failed: {err}"));
                            }
                        }
                        cx.stop_propagation();
                        cx.notify();
                    }))
                    .child("Go to definition"),
            );
        }
        let input_view = cx.entity();
        let input_focus = self.title_focus.clone();
        body = body
            .child(div().mt_2().text_color(rgb(MUTED)).child("Title text"))
            .child(
                div()
                    .id("title-draft")
                    .debug_selector(|| "inspector.title".into())
                    .relative()
                    .track_focus(&self.title_focus)
                    .px_2()
                    .py_1()
                    .border_1()
                    .border_color(rgb(TEAL))
                    .bg(rgb(0x0c0e12))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.title_focus.focus(window);
                        cx.notify();
                    }))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        handle_title_key(&mut this.title_draft, event);
                        let end = this.title_draft.encode_utf16().count();
                        this.title_selection_utf16 = end..end;
                        this.title_marked_utf16 = None;
                        cx.notify();
                    }))
                    .child(
                        canvas(
                            |_, _, _| (),
                            move |bounds, (), window, cx| {
                                window.handle_input(
                                    &input_focus,
                                    StudioTitleInputHandler {
                                        view: input_view,
                                        bounds,
                                    },
                                    cx,
                                );
                            },
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(if self.title_draft.is_empty() {
                        "(type to edit title)".to_string()
                    } else {
                        self.title_draft.clone()
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .mt_2()
                    .child(action_button("Apply edit", TEAL, cx, move |this, cx| {
                        let text = this.title_draft.clone();
                        if this.session.apply_title_text(&text).is_ok() {
                            this.adopt_locus_label();
                            this.after_edit();
                        }
                        cx.notify();
                    }))
                    .child(action_button("Review", LINE, cx, move |this, cx| {
                        let text = this.title_draft.clone();
                        let _ = this.session.propose_title_text(text);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("render-preview")
                    .debug_selector(|| "inspector.render-preview".into())
                    .mt_2()
                    .px_3()
                    .py_1()
                    .border_1()
                    .border_color(rgb(TEAL))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        match this.session.render_preview_with_renderer(this.renderer) {
                            Ok(report) => {
                                this.renderer_selection = Some(report.renderer.clone());
                                if !this.preview_retry_required {
                                    this.renderer_error = None;
                                }
                                this.last_render = Some(format!(
                                    "wrote {} ({})",
                                    report.output.display(),
                                    report.renderer
                                ));
                            }
                            Err(err) => {
                                trace::log(format!("render preview: {err}"));
                                if !this.preview_retry_required {
                                    this.renderer_error = Some(err.to_string());
                                }
                                this.last_render = Some(format!("render failed: {err}"));
                            }
                        }
                        cx.notify();
                    }))
                    .child("Render preview.mp4"),
            );
        if let Some(path) = &self.last_render {
            body = body.child(div().text_color(rgb(MUTED)).child(format!("wrote {path}")));
        }

        if let Some(review) = review {
            body = body
                .child(div().mt_3().text_color(rgb(TEAL)).child("Review"))
                .child(review.description.clone())
                .child(review.vel_diff.clone())
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .child(review_button("Apply", TEAL, cx, true))
                        .child(review_button("Reject", 0xc45c5c, cx, false)),
                );
        }
        pane("Inspector", px(240.0), body)
    }

    fn timeline_bar(
        &self,
        layout: Option<&lattice_studio::StudioLayout>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let Some(layout) = layout else {
            return div().h(px(140.0)).child("no timeline");
        };
        let mut tracks = div().flex().flex_col().gap_1().p_2();
        for track in &layout.timeline.tracks {
            tracks = tracks.child(self.track_row(track, layout, cx));
        }
        div()
            .h(px(160.0))
            .border_t_1()
            .border_color(rgb(LINE))
            .bg(rgb(PANEL))
            .child(
                div().px_2().py_1().child(
                    div()
                        .id("timeline-ruler")
                        .debug_selector(|| "timeline.ruler".into())
                        // Match the ruler's test/interaction bounds to the track rail:
                        // the track row reserves 56 px for its label plus an 8 px gap.
                        .relative()
                        .ml(px(64.0))
                        .w(px(TIMELINE_WIDTH))
                        .text_color(rgb(MUTED))
                        .cursor(CursorStyle::IBeam)
                        .child({
                            let geom = Arc::clone(&self.ruler_geom);
                            canvas(
                                move |bounds, _, _| {
                                    if let Ok(mut slot) = geom.lock() {
                                        *slot = Some((
                                            f32::from(bounds.origin.x),
                                            f32::from(bounds.origin.y),
                                            f32::from(bounds.size.width),
                                            f32::from(bounds.size.height),
                                        ));
                                    }
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .size_full()
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                let x = this.pointer_x(event.position.x);
                                this.apply_rail_width();
                                this.session.begin_timeline_scrub(x, event.modifiers.alt);
                                this.play_origin = None;
                                this.audio_play_pending = false;
                                this.sync_audio_monitor("timeline-scrub");
                                this.preview_inbox.clear_pending();
                                trace::log(format!(
                                    "gesture begin-scrub x={x:.1} playhead={}",
                                    format_time(this.session.playhead())
                                ));
                                this.last_inflight_key = None;
                                this.log_semantic_state("timeline-pointer-begin", None);
                                this.refresh_preview("timeline-clip");
                                this.queue_preview();
                                cx.notify();
                            }),
                        )
                        .child(format!("Timeline · {}", format_time(layout.playhead))),
                ),
            )
            .child(tracks)
    }

    fn track_row(
        &self,
        track: &lattice_studio::TimelineTrackView,
        layout: &lattice_studio::StudioLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let row = div()
            .flex()
            .flex_row()
            .items_center()
            .h(px(28.0))
            .gap_2()
            .child(
                div()
                    .w(px(56.0))
                    .text_color(rgb(MUTED))
                    .child(track.name.clone()),
            );
        let viewport = self.session.viewport();
        let playhead_x = viewport.x_at_time(self.session.playhead()) as f32;
        let geom = Arc::clone(&self.rail_geom);
        let tracks = Arc::clone(&self.track_geoms);
        let track_name = track.name.clone();
        let rail_id = SharedString::from(format!("timeline-rail-{}", track.name));
        let mut rail = div()
            .id(rail_id)
            .debug_selector({
                let track_name = track_name.clone();
                move || format!("timeline.track.{}", selector_component(&track_name))
            })
            .relative()
            .w(px(TIMELINE_WIDTH))
            .h(px(22.0))
            .bg(rgb(0x1a1f28))
            .child(
                canvas(
                    {
                        let track_name = track_name.clone();
                        move |bounds, _, _| {
                            let x = f32::from(bounds.origin.x);
                            let y = f32::from(bounds.origin.y);
                            let w = f32::from(bounds.size.width);
                            let h = f32::from(bounds.size.height);
                            if let Ok(mut slot) = geom.lock() {
                                *slot = (x, w);
                            }
                            if let Ok(mut slots) = tracks.lock() {
                                slots.retain(|(name, _, _, _, _)| name != &track_name);
                                slots.push((track_name.clone(), x, y, w, h));
                            }
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .capture_any_mouse_down(cx.listener({
                let track_name = track_name.clone();
                move |this, event: &MouseDownEvent, _, cx| {
                    if event.button != MouseButton::Left {
                        return;
                    }
                    this.begin_timeline_pointer_on(
                        event.position.x,
                        event.modifiers.alt,
                        &track_name,
                    );
                    this.queue_preview();
                    cx.notify();
                }
            }));
        for clip in &track.clips {
            let width = viewport.delta_x(clip.duration).abs().max(4.0) as f32;
            let left = viewport.x_at_time(clip.start) as f32;
            let id = clip.id.clone();
            let selected = clip.selected;
            let label = clip.label.clone();
            let handles = clip.handles;
            let color = match clip.track.as_str() {
                "text" => TEAL,
                "audio" => 0x5a7a9a,
                _ => 0x4a3a6a,
            };
            let mut block = div()
                .id(SharedString::from(format!("tl-{id}")))
                .debug_selector({
                    let id = id.clone();
                    move || format!("timeline.clip.{id}")
                })
                .absolute()
                .left(px(left))
                .top(px(0.0))
                .h_full()
                .w(px(width))
                .px_1()
                .bg(rgb(color))
                .border_1()
                .border_color(if selected { rgb(0xffffff) } else { rgb(color) })
                .child(label);
            if handles {
                block = block
                    .child(
                        div()
                            .id(SharedString::from(format!("tl-{id}-in")))
                            .debug_selector({
                                let id = id.clone();
                                move || format!("timeline.trim.{id}.in")
                            })
                            .absolute()
                            .left(px(0.0))
                            .top(px(0.0))
                            .w(px(8.0))
                            .h_full()
                            .bg(rgb(0xffffff))
                            .cursor(CursorStyle::ResizeLeftRight),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("tl-{id}-out")))
                            .debug_selector({
                                let id = id.clone();
                                move || format!("timeline.trim.{id}.out")
                            })
                            .absolute()
                            .right(px(0.0))
                            .top(px(0.0))
                            .w(px(8.0))
                            .h_full()
                            .bg(rgb(0xffffff))
                            .cursor(CursorStyle::ResizeLeftRight),
                    );
            }
            rail = rail.child(block);
        }
        if track.name == "Video" {
            rail = rail.child(
                div()
                    .id("playhead")
                    .debug_selector(|| "timeline.playhead".into())
                    .absolute()
                    .left(px(playhead_x))
                    .top(px(0.0))
                    .w(px(2.0))
                    .h_full()
                    .bg(rgb(TEAL)),
            );
        } else {
            rail = rail.child(
                div()
                    .id(SharedString::from(format!("playhead-{}", track.name)))
                    .debug_selector({
                        let track_name = track.name.clone();
                        move || format!("timeline.playhead.{}", selector_component(&track_name))
                    })
                    .absolute()
                    .left(px(playhead_x))
                    .top(px(0.0))
                    .w(px(2.0))
                    .h_full()
                    .bg(rgb(TEAL)),
            );
        }
        if let Some(snap) = layout.timeline.snap_indicator {
            let snap_x = viewport.x_at_time(snap) as f32;
            rail = rail.child(
                div()
                    .id(SharedString::from(format!("snap-indicator-{}", track.name)))
                    .absolute()
                    .left(px(snap_x))
                    .top(px(0.0))
                    .w(px(1.0))
                    .h_full()
                    .bg(rgb(0xf0c040)),
            );
        }
        if let Some(mark) = layout.timeline.insertion_marker {
            let mark_x = viewport.x_at_time(mark) as f32;
            rail = rail.child(
                div()
                    .id(SharedString::from(format!("insert-marker-{}", track.name)))
                    .absolute()
                    .left(px(mark_x))
                    .top(px(0.0))
                    .w(px(2.0))
                    .h_full()
                    .bg(rgb(0xffffff)),
            );
        }
        row.child(rail)
    }
}

#[cfg(feature = "window")]
fn header_bar(file: &str) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .h(px(36.0))
        .px_3()
        .bg(rgb(PANEL))
        .border_b_1()
        .border_color(rgb(LINE))
        .child(div().text_color(rgb(TEAL)).child("Lattice"))
        .child(
            div()
                .ml_3()
                .text_color(rgb(MUTED))
                .child(format!("{file} · Scene demo")),
        )
}

#[cfg(feature = "window")]
fn pane_flex(title: &'static str, child: impl IntoElement) -> impl IntoElement {
    pane_inner(title, child).flex_1()
}

#[cfg(feature = "window")]
fn pane(title: &'static str, width: gpui::Pixels, child: impl IntoElement) -> impl IntoElement {
    pane_inner(title, child).w(width)
}

#[cfg(feature = "window")]
fn pane_inner(title: &'static str, child: impl IntoElement) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .h_full()
        .border_r_1()
        .border_color(rgb(LINE))
        .bg(rgb(PANEL))
        .child(
            div()
                .px_2()
                .py_1()
                .border_b_1()
                .border_color(rgb(LINE))
                .text_color(rgb(TEAL))
                .child(title),
        )
        .child(
            div()
                .id(SharedString::from(format!("pane-{title}")))
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .p_2()
                .child(child),
        )
}

#[cfg(feature = "window")]
fn tree_nodes(
    nodes: &[lattice_studio::TreeNode],
    cx: &mut Context<StudioView>,
) -> impl IntoElement {
    let mut col = div().flex().flex_col().gap_1();
    for node in nodes {
        col = col.child(tree_node(node, 0, cx));
    }
    col
}

#[cfg(feature = "window")]
fn tree_node(
    node: &lattice_studio::TreeNode,
    depth: u32,
    cx: &mut Context<StudioView>,
) -> impl IntoElement {
    let id = node.id.clone();
    let selected = node.selected;
    let label = format!("{} {}", node.kind, node.label);
    let mut block = div().flex().flex_col();
    block = block.child(
        div()
            .id(SharedString::from(format!("tree-{id}")))
            .pr_1()
            .pl(px(8.0 + 14.0 * depth as f32))
            .bg(if selected { rgb(0x1c3d3a) } else { rgb(PANEL) })
            .text_color(if selected { rgb(TEAL) } else { rgb(TEXT) })
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.session
                    .point_at(lattice_engine::LocusId::new(id.clone()));
                this.adopt_locus_label();
                this.refresh_preview("tree");
                cx.notify();
            }))
            .child(label),
    );
    for child in &node.children {
        block = block.child(tree_node(child, depth + 1, cx));
    }
    block
}

#[cfg(feature = "window")]
fn review_button(
    label: &'static str,
    color: u32,
    cx: &mut Context<StudioView>,
    apply: bool,
) -> impl IntoElement {
    div()
        .id(label)
        .debug_selector(move || {
            if apply {
                "review.apply".into()
            } else {
                "review.reject".into()
            }
        })
        .px_3()
        .py_1()
        .bg(rgb(color))
        .text_color(rgb(BG))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| {
            if let Some(proposal) = this.session.review_proposal().cloned() {
                if apply {
                    let _ = this.session.apply_review(&proposal);
                    this.after_edit();
                } else {
                    let _ = this.session.reject_review(&proposal);
                }
                cx.notify();
            }
        }))
        .child(label)
}

#[cfg(feature = "window")]
fn handle_title_key(draft: &mut String, event: &KeyDownEvent) {
    let key = event.keystroke.key.as_str();
    if key == "backspace" {
        draft.pop();
    }
}

fn action_button(
    label: &'static str,
    color: u32,
    cx: &mut Context<StudioView>,
    on_click: impl Fn(&mut StudioView, &mut Context<StudioView>) + 'static,
) -> impl IntoElement {
    div()
        .id(label)
        .debug_selector(move || action_selector(label).into())
        .px_3()
        .py_1()
        .bg(rgb(color))
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(label)
}

fn action_selector(label: &str) -> &'static str {
    match label {
        "Open Video…" => "toolbar.import",
        "Set In" => "toolbar.set-in",
        "Set Out" => "toolbar.set-out",
        "Split at Playhead" => "toolbar.split",
        "Delete Selected Clip" => "toolbar.delete-clip",
        "CPU" => "toolbar.renderer.cpu",
        "GPU DX12" => "toolbar.renderer.gpu-dx12",
        "Play" => "toolbar.play",
        "Pause" => "toolbar.pause",
        "Seek" => "toolbar.seek-start",
        "Scrub" => "toolbar.scrub",
        "Save" => "toolbar.save",
        "Undo" => "toolbar.undo",
        "Redo" => "toolbar.redo",
        "Resolve" => "toolbar.resolve",
        "Copy locus JSON" => "toolbar.copy-locus",
        "Gain -3 dB" => "toolbar.gain-minus-3",
        "Fade" => "toolbar.fade",
        "Zoom In" => "toolbar.zoom-in",
        "Zoom Out" => "toolbar.zoom-out",
        "Apply edit" => "inspector.apply",
        "Review" => "inspector.review",
        _ => "toolbar.unknown",
    }
}

fn selector_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn open_video_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("LATTICE_OPEN_VIDEO") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    #[cfg(windows)]
    {
        windows_pick_mp4()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

#[cfg(windows)]
fn windows_pick_mp4() -> Option<PathBuf> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Filter = 'Video (*.mp4)|*.mp4|All files (*.*)|*.*'; if ($d.ShowDialog() -eq 'OK') { $d.FileName }",
        ])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }
    let path = PathBuf::from(text);
    path.is_file().then_some(path)
}

const TIMELINE_WIDTH: f32 = 640.0;

fn format_time(time: lattice_engine::Time) -> String {
    let seconds = time.num() as f64 / time.den().max(1) as f64;
    format!("{seconds:.2}s")
}
