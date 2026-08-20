//! Lattice Studio entry. Always GPUI (`window` is the default crate feature).
#![allow(
    clippy::cast_precision_loss,
    clippy::map_unwrap_or,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::unreadable_literal,
    clippy::unused_self
)]

use std::path::PathBuf;
use std::process::ExitCode;

use gpui::{
    AppContext, Application, Bounds, Context, FocusHandle, Focusable, InteractiveElement,
    IntoElement, KeyDownEvent, MouseButton, MouseMoveEvent, ObjectFit, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, StyledImage, Timer, TitlebarOptions, Window,
    WindowBounds, WindowOptions, div, img, px, rgb, size,
};
use lattice_studio::{StudioSession, trace};

fn main() -> ExitCode {
    let log_path = trace::install();
    let path = std::env::args_os().nth(1).map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/gameplay-commentary/main.vel")
        },
        PathBuf::from,
    );
    trace::log(format!(
        "start exe={} cwd={} vel={} log={} preview={} rustc={}",
        std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".into()),
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "?".into()),
        path.display(),
        log_path.display(),
        if preview_extract_enabled() {
            "on"
        } else {
            "off"
        },
        option_env!("CARGO_PKG_VERSION").unwrap_or("dev"),
    ));
    if let Err(err) = window_main(path) {
        trace::log(format!("fatal: {err}"));
        return ExitCode::from(2);
    }
    trace::log("event loop returned (window closed)");
    ExitCode::SUCCESS
}

fn window_main(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    trace::log(format!("compile/open {}", path.display()));
    let session = StudioSession::open(&path).map_err(|err| {
        trace::log(format!("StudioSession::open failed: {err}"));
        err
    })?;
    trace::log(format!(
        "open ok dirty={} playhead={:?} diagnostics={}",
        session.is_dirty(),
        session.playhead(),
        session.diagnostics().len()
    ));
    spawn_preview_extract(path.clone());
    trace::log("Application::run");
    Application::new().run(move |cx| {
        let bounds = Bounds::centered(None, size(px(1400.0), px(840.0)), cx);
        trace::log("open_window");
        match cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Lattice Studio".into()),
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
                    let mut view = StudioView {
                        session,
                        title_draft: String::new(),
                        last_render: None,
                        focus: cx.focus_handle(),
                        first_paint_logged: false,
                        timeline_dragging: false,
                        preview_shown: false,
                    };
                    view.adopt_locus_label();
                    view.spawn_play_clock(cx);
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

fn spawn_preview_extract(path: PathBuf) {
    if !preview_extract_enabled() {
        trace::log("preview extract skipped (LATTICE_STUDIO_PREVIEW=0)");
        return;
    }
    match std::thread::Builder::new()
        .name("lattice-preview".into())
        .spawn(move || {
            trace::log("preview extract thread start");
            match StudioSession::open(&path).and_then(|session| session.cached_preview_frame()) {
                Ok(frame) => trace::log(format!("preview extract ok {}", frame.display())),
                Err(err) => trace::log(format!("preview extract err {err}")),
            }
        }) {
        Ok(_) => trace::log("preview extract thread spawned"),
        Err(err) => trace::log(format!("preview extract thread spawn failed: {err}")),
    }
}

fn preview_extract_enabled() -> bool {
    match std::env::var("LATTICE_STUDIO_PREVIEW") {
        Ok(value) => {
            let value = value.to_ascii_lowercase();
            !matches!(value.as_str(), "0" | "off" | "false" | "no")
        }
        Err(_) => true,
    }
}

struct StudioView {
    session: StudioSession,
    title_draft: String,
    last_render: Option<String>,
    focus: FocusHandle,
    first_paint_logged: bool,
    timeline_dragging: bool,
    preview_shown: bool,
}

impl StudioView {
    fn adopt_locus_label(&mut self) {
        if let Ok(Some(locus)) = self.session.current_locus() {
            self.title_draft = locus.label;
        }
    }

    fn spawn_play_clock(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                Timer::after(std::time::Duration::from_millis(100)).await;
                if this
                    .update(cx, |this, cx| {
                        if this.session.is_playing() {
                            this.session
                                .step_clock(lattice_engine::Time::milliseconds(100));
                            this.refresh_preview("clock");
                            cx.notify();
                        } else if !this.preview_shown && this.session.peek_preview_frame().is_some()
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

    fn scrub_timeline_ratio(&mut self, num: u32, den: u32, why: &str) {
        self.session.scrub_timeline_ratio(num, den);
        self.refresh_preview(why);
    }
}

impl Drop for StudioView {
    fn drop(&mut self) {
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
        }
        let file = layout
            .as_ref()
            .map(|item| item.file_label.clone())
            .unwrap_or_else(|| "main.vel".into());
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(BG))
            .text_color(rgb(TEXT))
            .text_sm()
            .child(header_bar(&file))
            .child(self.actions_bar(cx))
            .child(self.body(layout.as_ref(), cx))
            .child(self.timeline_bar(layout.as_ref(), cx))
    }
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
                cx.notify();
            }))
            .child(action_button("Set Out", LINE, cx, move |this, cx| {
                if let Err(err) = this.session.set_out_at_playhead() {
                    trace::log(format!("set out: {err}"));
                }
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
                    cx.notify();
                },
            ))
            .child(action_button("Play", TEAL, cx, move |this, cx| {
                this.session.play();
                trace::log("play");
                cx.notify();
            }))
            .child(action_button("Pause", LINE, cx, move |this, cx| {
                this.session.pause();
                trace::log("pause");
                cx.notify();
            }))
            .child(action_button("Seek", LINE, cx, move |this, cx| {
                this.session.seek(lattice_engine::Time::ZERO);
                this.refresh_preview("seek");
                cx.notify();
            }))
            .child(action_button("Scrub", LINE, cx, move |this, cx| {
                this.session.scrub(this.session.playhead());
                this.refresh_preview("scrub");
                cx.notify();
            }))
            .child(action_button("Save", TEAL, cx, move |this, cx| {
                let _ = this.session.save();
                cx.notify();
            }))
            .child(action_button("Gain -3 dB", LINE, cx, move |this, cx| {
                let _ = this.session.set_gain(-3);
                cx.notify();
            }))
            .child(action_button("Fade", LINE, cx, move |this, cx| {
                let _ = this
                    .session
                    .set_fade(lattice_engine::Time::milliseconds(500));
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
                spawn_preview_extract(self.session.path().to_path_buf());
            }
            Err(err) => {
                trace::log(format!("open_video failed: {err}"));
                self.last_render = Some(format!("open video: {err}"));
            }
        }
    }

    fn refresh_preview(&mut self, why: &str) {
        if !preview_extract_enabled() {
            return;
        }
        match self.session.cached_preview_frame() {
            Ok(path) => trace::log(format!("preview {why} {}", path.display())),
            Err(err) => trace::log(format!("preview {why} err {err}")),
        }
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
            .child(self.source_pane(layout))
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
        let mut stage = div()
            .flex()
            .flex_col()
            .flex_1()
            .m_3()
            .bg(rgb(0x1a2330))
            .border_1()
            .border_color(rgb(LINE))
            .relative();
        if let Some(path) = layout.canvas.preview_frame.clone() {
            if path.is_file() {
                let width = layout.canvas.preview_width as f32;
                let height = layout.canvas.preview_height as f32;
                stage = stage.child(
                    img(path)
                        .object_fit(ObjectFit::Contain)
                        .id("canvas-frame")
                        .w(px(width.max(1.0)))
                        .h(px(height.max(1.0))),
                );
            } else {
                trace::log(format!("preview path missing {}", path.display()));
                stage = stage.child(div().flex_1());
            }
        } else {
            stage = stage.child(div().flex_1());
        }
        for overlay in overlays {
            let id = overlay.locus_id.clone();
            let selected = overlay.selected;
            let text = overlay.text.clone();
            stage = stage.child(
                div()
                    .id(SharedString::from(format!("canvas-{id}")))
                    .h(px(64.0))
                    .mx_6()
                    .mb_4()
                    .px_3()
                    .flex()
                    .items_center()
                    .border_2()
                    .border_color(if selected { rgb(TEAL) } else { rgb(LINE) })
                    .bg(rgb(0x10161c))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let _ = this.session.point_from_canvas_overlay(&id);
                        this.adopt_locus_label();
                        this.refresh_preview("canvas");
                        cx.notify();
                    }))
                    .child(div().text_xl().text_color(rgb(0xffffff)).child(text)),
            );
        }
        pane_flex("Canvas", stage)
    }

    fn source_pane(&self, layout: &lattice_studio::StudioLayout) -> impl IntoElement {
        let highlight = layout
            .source
            .highlight
            .map(|span| format!("span line {}", span.line))
            .unwrap_or_default();
        pane(
            "VEL",
            px(280.0),
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_color(rgb(TEAL)).child(highlight))
                .child(layout.source.text.clone()),
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
                    .mt_2()
                    .px_3()
                    .py_1()
                    .bg(rgb(TEAL))
                    .text_color(rgb(BG))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        let _ = this.session.go_to_definition();
                        cx.notify();
                    }))
                    .child("Go to definition"),
            );
        }
        body = body
            .child(div().mt_2().text_color(rgb(MUTED)).child("Title text"))
            .child(
                div()
                    .id("title-draft")
                    .track_focus(&self.focus)
                    .px_2()
                    .py_1()
                    .border_1()
                    .border_color(rgb(TEAL))
                    .bg(rgb(0x0c0e12))
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        handle_title_key(&mut this.title_draft, event);
                        cx.notify();
                    }))
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
                    .mt_2()
                    .px_3()
                    .py_1()
                    .border_1()
                    .border_color(rgb(TEAL))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Ok(path) = this.session.render_preview() {
                            this.last_render = Some(path.display().to_string());
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
            tracks = tracks.child(self.track_row(track, layout.timeline.duration, cx));
        }
        div()
            .h(px(160.0))
            .border_t_1()
            .border_color(rgb(LINE))
            .bg(rgb(PANEL))
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_color(rgb(MUTED))
                    .child(format!("Timeline · {}", format_time(layout.playhead))),
            )
            .child(tracks)
    }

    fn track_row(
        &self,
        track: &lattice_studio::TimelineTrackView,
        total: lattice_engine::Time,
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
        let playhead_x = time_px(self.session.playhead(), total);
        let mut rail = div()
            .id("timeline-rail")
            .relative()
            .w(px(TIMELINE_WIDTH))
            .h(px(22.0))
            .bg(rgb(0x1a1f28));
        for clip in &track.clips {
            let width = time_px(clip.duration, total).max(4.0);
            let left = time_px(clip.start, total);
            let id = clip.id.clone();
            let selected = clip.selected;
            let label = clip.label.clone();
            let color = match clip.track.as_str() {
                "text" => TEAL,
                "audio" => 0x5a7a9a,
                _ => 0x4a3a6a,
            };
            rail = rail.child(
                div()
                    .id(SharedString::from(format!("tl-{id}")))
                    .absolute()
                    .left(px(left))
                    .top(px(0.0))
                    .h_full()
                    .w(px(width))
                    .px_1()
                    .bg(rgb(color))
                    .border_1()
                    .border_color(if selected { rgb(0xffffff) } else { rgb(color) })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session
                            .point_at(lattice_engine::LocusId::new(id.clone()));
                        this.adopt_locus_label();
                        this.refresh_preview("timeline-clip");
                        cx.notify();
                    }))
                    .child(label),
            );
        }
        rail = rail.child(
            div()
                .id("playhead")
                .absolute()
                .left(px(playhead_x))
                .top(px(0.0))
                .w(px(2.0))
                .h_full()
                .bg(rgb(TEAL)),
        );
        let mut hits = div()
            .id("timeline-hits")
            .absolute()
            .size_full()
            .flex()
            .flex_row();
        for index in 0..TIMELINE_SLICES {
            hits = hits.child(
                div()
                    .id(SharedString::from(format!("tl-hit-{index}")))
                    .flex_1()
                    .h_full()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.timeline_dragging = true;
                            this.scrub_timeline_ratio(index, TIMELINE_RATIO_DEN, "timeline-drag");
                            cx.notify();
                        }),
                    )
                    .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
                        if this.timeline_dragging || event.dragging() {
                            this.scrub_timeline_ratio(index, TIMELINE_RATIO_DEN, "timeline-drag");
                            cx.notify();
                        }
                    }))
                    .on_mouse_up(
                        MouseButton::Left,
                        cx.listener(move |this, _, _, cx| {
                            this.timeline_dragging = false;
                            let _ = this.session.click_timeline_ratio(index, TIMELINE_RATIO_DEN);
                            this.refresh_preview("timeline-drag");
                            cx.notify();
                        }),
                    ),
            );
        }
        rail = rail.child(hits);
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
        .px_3()
        .py_1()
        .bg(rgb(color))
        .text_color(rgb(BG))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| {
            if let Some(proposal) = this.session.review_proposal().cloned() {
                if apply {
                    let _ = this.session.apply_review(&proposal);
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
    match key {
        "backspace" => {
            draft.pop();
        }
        "space" => draft.push(' '),
        "enter" | "escape" | "tab" => {}
        k if k.chars().count() == 1 => {
            draft.push_str(k);
        }
        _ => {}
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
        .px_3()
        .py_1()
        .bg(rgb(color))
        .text_color(rgb(TEXT))
        .cursor_pointer()
        .on_click(cx.listener(move |this, _, _, cx| on_click(this, cx)))
        .child(label)
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
/// Hit cells `0..=TIMELINE_RATIO_DEN` so the last cell is duration (`num == den`).
const TIMELINE_RATIO_DEN: u32 = 100;
const TIMELINE_SLICES: u32 = TIMELINE_RATIO_DEN + 1;

fn time_px(span: lattice_engine::Time, total: lattice_engine::Time) -> f32 {
    let n = span.num() as f64 / span.den().max(1) as f64;
    let d = total.num() as f64 / total.den().max(1) as f64;
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    {
        ((n / d.max(0.001)) * f64::from(TIMELINE_WIDTH)).clamp(0.0, f64::from(TIMELINE_WIDTH))
            as f32
    }
}

fn format_time(time: lattice_engine::Time) -> String {
    let seconds = time.num() as f64 / time.den().max(1) as f64;
    format!("{seconds:.2}s")
}
