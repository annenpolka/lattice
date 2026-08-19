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
    IntoElement, KeyDownEvent, ParentElement, Render, SharedString, StatefulInteractiveElement,
    Styled, TitlebarOptions, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use lattice_studio::StudioSession;

fn main() -> ExitCode {
    let path = std::env::args_os().nth(1).map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/gameplay-commentary/main.vel")
        },
        PathBuf::from,
    );
    if let Err(err) = window_main(path) {
        eprintln!("lattice-studio: {err}");
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

fn window_main(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let session = StudioSession::open(&path)?;
    eprintln!("lattice-studio: opening GPUI window for {}", path.display());
    Application::new().run(move |cx| {
        let bounds = Bounds::centered(None, size(px(1400.0), px(840.0)), cx);
        cx.open_window(
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
                    let mut view = StudioView {
                        session,
                        title_draft: String::new(),
                        last_render: None,
                        focus: cx.focus_handle(),
                    };
                    view.adopt_locus_label();
                    view
                })
            },
        )
        .expect("open studio window");
        cx.activate(true);
    });
    Ok(())
}

struct StudioView {
    session: StudioSession,
    title_draft: String,
    last_render: Option<String>,
    focus: FocusHandle,
}

impl StudioView {
    fn adopt_locus_label(&mut self) {
        if let Ok(Some(locus)) = self.session.current_locus() {
            self.title_draft = locus.label;
        }
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
        let layout = self.session.layout().ok();
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
            .child(self.body(layout.as_ref(), cx))
            .child(self.timeline_bar(layout.as_ref(), cx))
    }
}

#[cfg(feature = "window")]
impl StudioView {
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
        stage = stage.child(div().flex_1());
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
            .child(div().px_2().py_1().text_color(rgb(MUTED)).child("Timeline"))
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
        let mut rail = div().flex().flex_1().h(px(22.0)).bg(rgb(0x1a1f28));
        for clip in &track.clips {
            let width = clip_px(clip.duration, total);
            let gap = clip_px(clip.start, total);
            let id = clip.id.clone();
            let selected = clip.selected;
            let label = clip.label.clone();
            let color = match clip.track.as_str() {
                "text" => TEAL,
                "audio" => 0x5a7a9a,
                _ => 0x4a3a6a,
            };
            rail = rail.child(div().w(px(gap.min(4.0)))).child(
                div()
                    .id(SharedString::from(format!("tl-{id}")))
                    .h_full()
                    .w(px(width.max(24.0)))
                    .px_1()
                    .bg(rgb(color))
                    .border_1()
                    .border_color(if selected { rgb(0xffffff) } else { rgb(color) })
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.session
                            .point_at(lattice_engine::LocusId::new(id.clone()));
                        this.adopt_locus_label();
                        cx.notify();
                    }))
                    .child(label),
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

fn clip_px(span: lattice_engine::Time, total: lattice_engine::Time) -> f32 {
    let n = span.num() as f64 / span.den().max(1) as f64;
    let d = total.num() as f64 / total.den().max(1) as f64;
    #[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    {
        ((n / d.max(0.001)) * 640.0).clamp(8.0, 640.0) as f32
    }
}
