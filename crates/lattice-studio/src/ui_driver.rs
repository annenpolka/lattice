//! Selector-driven GPUI interaction driver for Studio tests.
//!
//! This intentionally dispatches platform input through [`gpui::VisualTestContext`]. It never
//! calls a view handler directly and never stores absolute screen coordinates in a test.

use gpui::{
    App, Bounds, Entity, Modifiers, MouseButton, Pixels, Point, ScrollDelta, ScrollWheelEvent,
    Size, VisualTestContext, point, px,
};

const DRAG_STEPS: usize = 5;

fn secondary_keystroke(key: &str) -> String {
    let modifier = if cfg!(target_os = "macos") {
        "cmd"
    } else {
        "ctrl"
    };
    format!("{modifier}-{key}")
}

pub(crate) struct UiDriver<'a> {
    cx: &'a mut VisualTestContext,
}

impl<'a> UiDriver<'a> {
    pub(crate) fn new(cx: &'a mut VisualTestContext) -> Self {
        Self { cx }
    }

    pub(crate) fn context(&mut self) -> &mut VisualTestContext {
        self.cx
    }

    pub(crate) fn read<V: 'static, R>(
        &mut self,
        entity: &Entity<V>,
        read: impl FnOnce(&V, &App) -> R,
    ) -> R {
        entity.read_with(self.cx, read)
    }

    pub(crate) fn bounds(&mut self, selector: impl Into<String>) -> Bounds<Pixels> {
        let selector = selector.into();
        let stable: &'static str = Box::leak(selector.clone().into_boxed_str());
        self.cx
            .debug_bounds(stable)
            .unwrap_or_else(|| panic!("missing rendered UI selector `{selector}`"))
    }

    pub(crate) fn point_at(
        &mut self,
        selector: impl Into<String>,
        relative_x: f32,
        relative_y: f32,
    ) -> Point<Pixels> {
        let bounds = self.bounds(selector);
        point(
            px(f32::from(bounds.origin.x) + f32::from(bounds.size.width) * relative_x),
            px(f32::from(bounds.origin.y) + f32::from(bounds.size.height) * relative_y),
        )
    }

    pub(crate) fn click(&mut self, selector: impl Into<String>) {
        let at = self.point_at(selector, 0.5, 0.5);
        self.cx.simulate_click(at, Modifiers::none());
    }

    pub(crate) fn click_at(
        &mut self,
        selector: impl Into<String>,
        relative_x: f32,
        relative_y: f32,
    ) {
        let at = self.point_at(selector, relative_x, relative_y);
        self.cx.simulate_click(at, Modifiers::none());
    }

    pub(crate) fn drag(
        &mut self,
        source: impl Into<String>,
        source_point: (f32, f32),
        target: impl Into<String>,
        target_point: (f32, f32),
    ) {
        let from = self.point_at(source, source_point.0, source_point.1);
        let to = self.point_at(target, target_point.0, target_point.1);
        self.drag_points(from, to);
    }

    pub(crate) fn drag_within(
        &mut self,
        selector: impl Into<String>,
        from: (f32, f32),
        to: (f32, f32),
    ) {
        let selector = selector.into();
        let from = self.point_at(selector.clone(), from.0, from.1);
        let to = self.point_at(selector, to.0, to.1);
        self.drag_points(from, to);
    }

    fn drag_points(&mut self, from: Point<Pixels>, to: Point<Pixels>) {
        self.cx.simulate_mouse_move(from, None, Modifiers::none());
        self.cx
            .simulate_mouse_down(from, MouseButton::Left, Modifiers::none());
        for step in 1..=DRAG_STEPS {
            let ratio = step as f32 / DRAG_STEPS as f32;
            let at = point(
                px(f32::from(from.x) + (f32::from(to.x) - f32::from(from.x)) * ratio),
                px(f32::from(from.y) + (f32::from(to.y) - f32::from(from.y)) * ratio),
            );
            self.cx
                .simulate_mouse_move(at, MouseButton::Left, Modifiers::none());
        }
        self.cx
            .simulate_mouse_up(to, MouseButton::Left, Modifiers::none());
    }

    pub(crate) fn type_text(&mut self, selector: impl Into<String>, text: &str) {
        self.click(selector);
        if !text.is_empty() {
            self.cx.simulate_input(text);
        }
    }

    /// Send text through the currently focused GPUI `InputHandler` without changing selection.
    pub(crate) fn input_text(&mut self, text: &str) {
        if !text.is_empty() {
            self.cx.simulate_input(text);
        }
    }

    pub(crate) fn press(&mut self, keystrokes: &str) {
        self.cx.simulate_keystrokes(keystrokes);
    }

    pub(crate) fn scroll(
        &mut self,
        selector: impl Into<String>,
        delta_x: f32,
        delta_y: f32,
        secondary: bool,
    ) {
        let position = self.point_at(selector, 0.5, 0.5);
        self.cx.simulate_event(ScrollWheelEvent {
            position,
            delta: ScrollDelta::Pixels(point(px(delta_x), px(delta_y))),
            modifiers: if secondary {
                Modifiers::secondary_key()
            } else {
                Modifiers::none()
            },
            ..Default::default()
        });
    }

    pub(crate) fn resize(&mut self, width: f32, height: f32) {
        self.cx.simulate_resize(Size {
            width: px(width),
            height: px(height),
        });
        self.cx.run_until_parked();
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use gpui::{Focusable as _, TestAppContext};
    use lattice_engine::{
        LocusId, RawFrame, RendererBackend, RendererRequest, RendererSelection, Time,
    };

    use super::{UiDriver, secondary_keystroke};
    use crate::{StudioView, render_image_from_raw};

    fn fixture_session(tag: &str) -> lattice_studio::StudioSession {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("lattice-ui-{tag}-{nonce}"));
        std::fs::create_dir_all(&root).expect("fixture dir");
        lattice_media::generate_av_fixture(root.join("capture.mp4"), 6).expect("A/V fixture");
        let vel = root.join("main.vel");
        std::fs::write(
            &vel,
            r#"project "ui-driver"
convention commentary
media game "capture.mp4"
sequence main {
  intro
}
scene intro {
  game[0s..6s] as clip
  title "Hello" {
    at 1s for 3s
  }
}
"#,
        )
        .expect("write fixture");
        lattice_studio::StudioSession::open(vel).expect("open fixture")
    }

    fn source_navigation_session() -> lattice_studio::StudioSession {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("lattice-ui-source-{nonce}"));
        std::fs::create_dir_all(&root).expect("fixture dir");
        lattice_media::generate_av_fixture(root.join("capture.mp4"), 6).expect("A/V fixture");
        let mut source = r#"project "source-navigation"
convention commentary
media game "capture.mp4"
sequence main {
  intro
}
scene intro {
  game[0s..6s] as clip
  callout "Near" {
    at 1s for 3s
  }
"#
        .to_string();
        for index in 0..64 {
            writeln!(source, "  // source navigation filler {index}")
                .expect("write source fixture");
        }
        source.push_str(
            r#"  title "Hello" {
    at 1s for 3s
  }
"#,
        );
        for index in 0..32 {
            writeln!(source, "  // trailing source filler {index}").expect("write source fixture");
        }
        source.push_str("}\n");
        let vel = root.join("main.vel");
        std::fs::write(&vel, source).expect("write fixture");
        lattice_studio::StudioSession::open(vel).expect("open fixture")
    }

    fn add_studio(
        cx: &mut TestAppContext,
        session: lattice_studio::StudioSession,
    ) -> (gpui::Entity<StudioView>, &mut gpui::VisualTestContext) {
        let (view, cx) = cx.add_window_view(move |_, cx| {
            let mut view = StudioView::new(
                session,
                RendererRequest::RequireCpu,
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
                cx.focus_handle(),
            );
            view.adopt_locus_label();
            view.preview_current = Some(
                render_image_from_raw(&RawFrame::filled(960, 540, 20, 30, 40, 255))
                    .expect("test image"),
            );
            view
        });
        cx.update(|window, app| {
            view.read(app).focus_handle(app).focus(window);
            window.activate_window();
        });
        (view, cx)
    }

    #[gpui::test]
    fn selector_click_text_review_undo_and_resize(cx: &mut TestAppContext) {
        let mut session = fixture_session("commands");
        let _ = session.point_at_title().expect("title locus");
        let original = session.source().to_string();
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);

        let focus = ui.read(&view, gpui::Focusable::focus_handle);
        assert!(
            ui.context().update(|window, _| focus.is_focused(window)),
            "Studio must receive initial keyboard focus"
        );

        ui.type_text("inspector.title", " ui");
        ui.click("inspector.review");
        assert!(
            ui.read(&view, |view, _| view.session.review_proposal().is_some()),
            "Review must be reached through GPUI click dispatch"
        );
        ui.click("review.reject");
        assert!(ui.read(&view, |view, _| view.session.review_proposal().is_none()));
        assert_eq!(
            ui.read(&view, |view, _| view.session.source().to_string()),
            original
        );

        ui.click("inspector.review");
        assert!(ui.read(&view, |view, _| view.session.review_proposal().is_some()));
        ui.click("review.apply");
        assert!(ui.read(&view, |view, _| view.session.review_proposal().is_none()));
        let reviewed = ui.read(&view, |view, _| view.session.source().to_string());
        assert_ne!(
            reviewed, original,
            "Review Apply must commit through GPUI click dispatch"
        );

        ui.type_text("inspector.title", " edited");
        ui.click("inspector.apply");
        assert_ne!(
            ui.read(&view, |view, _| view.session.source().to_string()),
            reviewed
        );
        assert_eq!(ui.read(&view, |view, _| view.session.undo_len()), 1);
        ui.press(&secondary_keystroke("z"));
        assert_eq!(
            ui.read(&view, |view, _| view.session.source().to_string()),
            reviewed
        );

        let before = ui.read(&view, |view, _| view.session.viewport().visible_duration());
        ui.resize(1180.0, 760.0);
        let _ = ui.bounds("toolbar.zoom-in");
        ui.click_at("toolbar.zoom-in", 0.5, 0.5);
        let after = ui.read(&view, |view, _| view.session.viewport().visible_duration());
        assert!(after < before, "selector click must survive window resize");

        let before_scroll = after;
        ui.scroll("timeline.ruler", 0.0, -40.0, true);
        assert!(
            ui.read(&view, |view, _| {
                view.session.viewport().visible_duration()
            }) < before_scroll,
            "control-scroll must use the actual root event route"
        );
    }

    #[gpui::test]
    fn vel_pane_click_navigate_and_edit_use_shared_locus(cx: &mut TestAppContext) {
        let session = source_navigation_session();
        let loci = session.loci().expect("loci");
        let callout = loci
            .iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Callout)
            .expect("callout")
            .clone();
        let title = loci
            .iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
            .expect("title")
            .clone();
        let callout_line = callout.source_span.expect("callout span").line;
        let title_span = title.source_span.expect("title span");
        let title_line = title_span.line;
        let title_selector = format!("canvas.overlay.{}", title.id.as_str());
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);

        ui.click_at(format!("vel.line.{callout_line}"), 0.15, 0.5);
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session
                    .current_locus()
                    .expect("current")
                    .expect("locus")
                    .id
            }),
            callout.id,
            "real VEL row click must map its source offset to the shared locus"
        );

        ui.click(title_selector);
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session
                    .current_locus()
                    .expect("current")
                    .expect("locus")
                    .id
            }),
            title.id
        );
        ui.click("vel.go-to-definition");
        ui.context().run_until_parked();

        let source_focus = ui.read(&view, |view, _| view.source_focus.clone());
        assert!(
            ui.context()
                .update(|window, _| source_focus.is_focused(window)),
            "Go to definition must focus the VEL input surface"
        );
        let (draft, selection, top_item) = ui.read(&view, |view, _| {
            (
                view.source_draft.clone(),
                view.source_selection_utf16.clone(),
                view.source_scroll.top_item(),
            )
        });
        let expected_selection = crate::byte_range_to_utf16(
            &draft,
            usize::try_from(title_span.start).unwrap()..usize::try_from(title_span.end).unwrap(),
        );
        assert_eq!(selection, expected_selection);
        let title_item = usize::try_from(title_line).unwrap() - 1;
        assert!(
            top_item <= title_item && title_item - top_item <= 1,
            "Go to definition must reveal the selected source line (top={top_item}, target={title_item})"
        );

        ui.input_text("title \"Edited from VEL\" {\n    at 1s for 3s\n  }");
        let (source, draft, current, undo_len, dirty, errors) = ui.read(&view, |view, _| {
            (
                view.session.source().to_string(),
                view.source_draft.clone(),
                view.session
                    .current_locus()
                    .expect("current")
                    .expect("locus"),
                view.session.undo_len(),
                view.session.is_dirty(),
                view.session.diagnostics().len(),
            )
        });
        assert!(source.contains("Edited from VEL"));
        assert_eq!(draft, source);
        assert_eq!(current.kind, lattice_engine::LocusKind::Title);
        assert_eq!(current.label, "Edited from VEL");
        assert!(
            undo_len > 0,
            "successful source input callbacks must enter session undo"
        );
        assert!(dirty);
        assert_eq!(errors, 0, "valid VEL input must recompile immediately");

        let valid_source = source;
        ui.press(&secondary_keystroke("a"));
        ui.input_text("@ broken\ncallout \"highlight survives\"");
        let (invalid_draft, committed_source, source_error, highlights) =
            ui.read(&view, |view, _| {
                (
                    view.source_draft.clone(),
                    view.session.source().to_string(),
                    view.source_error.clone(),
                    view.session.engine().highlight_vel(&view.source_draft),
                )
            });
        assert!(invalid_draft.starts_with('@'));
        assert_eq!(
            committed_source, valid_source,
            "invalid draft must not replace the last valid compiled source"
        );
        assert!(
            source_error.is_some(),
            "invalid draft must remain observable"
        );
        assert!(highlights.iter().any(|token| {
            token.text == "@" && token.class == lattice_engine::VelHighlightClass::Invalid
        }));
        assert!(highlights.iter().any(|token| {
            token.text == "callout" && token.class == lattice_engine::VelHighlightClass::Builtin
        }));
    }

    #[gpui::test]
    fn relative_scrub_and_trim_dispatch_real_drag_events(cx: &mut TestAppContext) {
        let mut session = fixture_session("timeline");
        session.point_at(LocusId::new("scene:intro"));
        let initial_layout = session.layout().expect("layout");
        let video = initial_layout
            .timeline
            .tracks
            .iter()
            .find(|track| track.name == "Video")
            .and_then(|track| track.clips.first())
            .expect("video clip")
            .clone();
        let clip_selector = format!("timeline.clip.{}", video.id);
        let trim_selector = format!("timeline.trim.{}.out", video.id);
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);

        ui.drag_within("timeline.ruler", (0.15, 0.5), (0.65, 0.5));
        let scrubbed = ui.read(&view, |view, _| view.session.playhead());
        assert!(
            scrubbed > Time::seconds(2) && scrubbed < Time::seconds(5),
            "relative ruler drag must move the model playhead, got {scrubbed}"
        );

        // Trim hits only selected-clip drawn handles. Point the source after
        // scrub so `timeline.trim.*.out` is both rendered and hittable.
        ui.click(&clip_selector);
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session
                    .current_locus()
                    .unwrap()
                    .map(|locus| locus.kind)
            }),
            Some(lattice_engine::LocusKind::Source)
        );
        let original = ui.read(&view, |view, _| view.session.source().to_string());
        ui.drag(trim_selector, (0.5, 0.5), clip_selector, (0.72, 0.5));
        assert_ne!(
            ui.read(&view, |view, _| view.session.source().to_string()),
            original,
            "trim must commit through mouse down/move/up"
        );
        assert_eq!(ui.read(&view, |view, _| view.session.undo_len()), 1);
    }

    #[gpui::test]
    fn corner_handle_resize_is_selector_relative_and_source_backed(cx: &mut TestAppContext) {
        let mut session = fixture_session("canvas-resize");
        let title = session
            .point_at_title()
            .expect("title locus")
            .expect("title");
        let original = session.source().to_string();
        let before = session
            .layout()
            .expect("layout")
            .canvas
            .overlays
            .into_iter()
            .find(|overlay| overlay.locus_id == title.id.as_str())
            .expect("selected overlay");
        let overlay_selector = format!("canvas.overlay.{}", title.id.as_str());
        let handle_selector = format!("canvas.resize.{}.top-right", title.id.as_str());
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);

        ui.drag(handle_selector, (0.5, 0.5), overlay_selector, (1.25, -0.25));
        let (after, source_has_scale, undo_len) = ui.read(&view, |view, _| {
            let after = view
                .session
                .layout()
                .expect("layout after resize")
                .canvas
                .overlays
                .into_iter()
                .find(|overlay| overlay.locus_id == title.id.as_str())
                .expect("resized overlay");
            (
                after,
                view.session.source().contains("scale "),
                view.session.undo_len(),
            )
        });
        assert!(after.width > before.width && after.height > before.height);
        assert_eq!(
            (after.x, i64::from(after.y) + i64::from(after.height)),
            (before.x, i64::from(before.y) + i64::from(before.height)),
            "top-right resize must keep the opposite corner fixed"
        );
        assert!(source_has_scale);
        assert_eq!(undo_len, 1);

        ui.press(&secondary_keystroke("z"));
        assert_eq!(
            ui.read(&view, |view, _| view.session.source().to_string()),
            original
        );
    }

    #[gpui::test]
    fn current_preview_failure_keeps_last_still_and_requires_explicit_retry(
        cx: &mut TestAppContext,
    ) {
        let session = fixture_session("preview-failure");
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);

        let (good_current, good_previous) = view.update(ui.context(), |view, cx| {
            let job = view
                .session
                .request_preview_job_with_renderer(RendererRequest::RequireCpu);
            let result = Ok((
                RawFrame::filled(960, 540, 80, 90, 100, 255),
                RendererSelection {
                    requested: RendererRequest::RequireCpu,
                    active: Some(RendererBackend::Cpu),
                    adapter: None,
                    reason: "test CPU renderer".into(),
                },
            ));
            *view.preview_slot.lock().expect("preview slot") =
                Some((job.generation, result, job.timeline_time, job.stamp));
            assert!(view.drain_preview(), "good frame must publish first");
            cx.notify();
            (
                view.preview_current.clone().expect("current good frame"),
                view.preview_previous.clone().expect("previous good frame"),
            )
        });
        ui.context().run_until_parked();
        let _ = ui.bounds("canvas.frame");

        view.update(ui.context(), |view, cx| {
            view.session.play();
            view.play_origin = Some((std::time::Instant::now(), view.session.playhead()));
            let stale = view
                .session
                .request_preview_job_with_renderer(RendererRequest::RequireCpu);
            let job = view
                .session
                .request_preview_job_with_renderer(RendererRequest::RequireCpu);
            *view.preview_slot.lock().expect("preview slot") = Some((
                stale.generation,
                Err("stale renderer error (test)".into()),
                stale.timeline_time,
                stale.stamp,
            ));
            assert!(
                !view.drain_preview(),
                "stale renderer error must be ignored"
            );
            assert!(view.session.is_playing());
            assert!(view.renderer_error.is_none());
            let _ = view.preview_inbox.push(job.clone());
            assert_eq!(view.preview_inbox.stats().pending, 1);
            *view.preview_slot.lock().expect("preview slot") = Some((
                job.generation,
                Err("renderer device lost (test)".into()),
                job.timeline_time,
                job.stamp,
            ));
            assert!(view.drain_preview(), "current renderer error must surface");
            cx.notify();
        });
        ui.context().run_until_parked();

        let _ = ui.bounds("toolbar.renderer-status");
        ui.read(&view, |view, _| {
            assert!(!view.session.is_playing(), "renderer error must pause");
            assert!(view.play_origin.is_none(), "play clock origin must clear");
            assert!(!view.audio_play_pending);
            assert_eq!(view.preview_inbox.stats().pending, 0);
            assert!(view.preview_retry_required);
            assert_eq!(
                view.renderer_error.as_deref(),
                Some("renderer device lost (test)")
            );
            assert!(
                view.renderer_status()
                    .contains("renderer device lost (test)")
            );
            assert!(std::sync::Arc::ptr_eq(
                &good_current,
                view.preview_current.as_ref().expect("retained current")
            ));
            assert!(std::sync::Arc::ptr_eq(
                &good_previous,
                view.preview_previous.as_ref().expect("retained previous")
            ));
            assert_eq!(view.session.preview_mailbox().retained_frame_count(), 1);
        });

        view.update(ui.context(), |view, _| {
            view.refresh_preview("automatic-after-error");
            view.queue_preview();
        });
        ui.click("toolbar.play");
        assert!(ui.read(&view, |view, _| {
            view.preview_retry_required
                && !view.session.is_playing()
                && view.preview_inbox.stats().pending == 0
        }));

        ui.click("toolbar.renderer.cpu");
        ui.read(&view, |view, _| {
            assert!(!view.preview_retry_required);
            assert!(view.renderer_error.is_none());
            assert_eq!(view.preview_inbox.stats().pending, 1);
            let generation = view.session.preview_mailbox().current_generation();
            assert!(
                view.preview_inbox.take_sampler_reset(generation),
                "explicit same-renderer selection must recreate the sampler"
            );
        });
    }

    #[gpui::test]
    fn headless_audio_play_waits_for_pcm_and_transport_cancels_pending(cx: &mut TestAppContext) {
        let session = fixture_session("audio-pending");
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);

        assert!(ui.read(&view, |view, _| !view.audio_enabled));
        assert!(ui.read(&view, |view, _| view.audio_format.is_none()));

        ui.click("toolbar.play");
        let (playing, pending, monitor_ready, device_was_probed) = ui.read(&view, |view, _| {
            (
                view.session.is_playing(),
                view.audio_play_pending,
                view.audio_monitor.is_some(),
                view.audio_format.is_some(),
            )
        });
        assert!(!playing, "video must not run ahead of AudioPlan PCM");
        assert!(pending, "Play must wait for the audio prepare result");
        assert!(!monitor_ready);
        assert!(
            !device_was_probed,
            "headless Studio must not touch the platform output device"
        );

        for stop_action in ["toolbar.seek-start", "toolbar.scrub", "toolbar.pause"] {
            ui.click(stop_action);
            assert!(ui.read(&view, |view, _| {
                !view.audio_play_pending && !view.session.is_playing() && view.play_origin.is_none()
            }));
            if stop_action != "toolbar.pause" {
                ui.click("toolbar.play");
                assert!(ui.read(&view, |view, _| view.audio_play_pending));
            }
        }
    }

    fn overlap_session() -> lattice_studio::StudioSession {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("lattice-ui-overlap-{nonce}"));
        std::fs::create_dir_all(&root).expect("dir");
        lattice_media::generate_av_fixture(root.join("capture.mp4"), 8).expect("fixture");
        let vel = root.join("main.vel");
        std::fs::write(
            &vel,
            r#"project "overlap"
convention commentary
media game "capture.mp4"
sequence main {
  demo
}
scene demo {
  game[0s..6s] as fight
  title "Hello" {
    at 2s for 3s
  }
}
"#,
        )
        .expect("write");
        lattice_studio::StudioSession::open(vel).expect("open")
    }

    #[gpui::test]
    fn video_clip_click_keeps_source_and_hides_title_fields(cx: &mut TestAppContext) {
        let session = overlap_session();
        let clip_id = session
            .layout()
            .unwrap()
            .timeline
            .tracks
            .iter()
            .find(|track| track.name == "Video")
            .unwrap()
            .clips[0]
            .id
            .clone();
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);
        ui.click(format!("timeline.clip.{clip_id}"));
        let (kind, title_fields, heading) = ui.read(&view, |view, _| {
            let layout = view.session.layout().unwrap();
            (
                view.session
                    .current_locus()
                    .unwrap()
                    .map(|locus| locus.kind),
                layout.inspector.title_fields,
                layout.inspector.heading,
            )
        });
        assert_eq!(kind, Some(lattice_engine::LocusKind::Source));
        assert!(!title_fields, "title fields only when here is Title");
        assert!(heading.contains("source"), "{heading}");
        let spoken = ui.read(&view, |view, _| view.session.utterance().spoken_text());
        assert!(
            spoken.contains("split →") && spoken.contains("do not retarget"),
            "{spoken}"
        );
    }

    #[gpui::test]
    fn overlap_candidates_are_on_timeline_and_toolbar_speaks(cx: &mut TestAppContext) {
        let session = overlap_session();
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);
        let ratio = ui.read(&view, |view, _| {
            let at = lattice_engine::Time::from_decimal_seconds(2, 4, 1).unwrap();
            let width = view.session.viewport().width_pixels();
            (view.session.x_at_time(at) / width) as f32
        });
        ui.click_at("timeline.track.audio", ratio, 0.5);
        let (kinds, title) = ui.read(&view, |view, _| {
            let unresolved = view
                .session
                .unresolved_pointing()
                .expect("empty-rail click must open unresolved pointing");
            assert_eq!(unresolved.projection, lattice_studio::Projection::Timeline);
            let kinds: Vec<_> = unresolved
                .candidates
                .iter()
                .map(|locus| locus.kind)
                .collect();
            let title = unresolved
                .candidates
                .iter()
                .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
                .expect("title candidate")
                .id
                .clone();
            (kinds, title)
        });
        assert!(
            kinds.contains(&lattice_engine::LocusKind::Title),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&lattice_engine::LocusKind::Source),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&lattice_engine::LocusKind::Scene),
            "{kinds:?}"
        );
        let _ = ui.bounds("timeline.candidates");
        ui.click(format!("timeline.candidate.{}", title.as_str()));
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session.current_locus().unwrap().unwrap().id
            }),
            title
        );
        assert!(ui.read(&view, |view, _| {
            view.session.layout().unwrap().inspector.title_fields
        }));

        let spoken = ui.read(&view, |view, _| view.session.utterance().spoken_text());
        assert!(
            !spoken.is_empty() && spoken.contains("title →"),
            "utterance discloses Title legality without a toolbar commit: {spoken}"
        );
        assert!(
            spoken.contains("delete →") && spoken.contains("committed on Toolbar"),
            "selected-clip Delete is disclosed as a toolbar commit: {spoken}"
        );
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session.current_locus().unwrap().unwrap().id
            }),
            title,
            "disclosure must not retarget here"
        );
        assert!(
            ui.read(&view, |view, _| {
                view.session
                    .layout()
                    .unwrap()
                    .timeline
                    .tracks
                    .iter()
                    .flat_map(|track| track.clips.iter())
                    .all(|clip| !clip.cut_lane)
            }),
            "cut lane is not drawn when here is Title"
        );
    }

    #[gpui::test]
    fn scene_inspector_has_no_title_selector(cx: &mut TestAppContext) {
        let session = overlap_session();
        let scene = session
            .loci()
            .unwrap()
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Scene)
            .unwrap()
            .id;
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);
        ui.click(format!("tree.node.{}", scene.as_str()));
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session.current_locus().unwrap().map(|locus| locus.id)
            }),
            Some(scene.clone())
        );
        assert!(ui.read(&view, |view, _| {
            !view.session.layout().unwrap().inspector.title_fields
        }));
        let spoken = ui.read(&view, |view, _| view.session.utterance().spoken_text());
        assert!(
            spoken.contains("needs-source-binding") || spoken.contains("Point the video clip"),
            "{spoken}"
        );
        assert!(
            ui.read(&view, |view, _| {
                view.session
                    .layout()
                    .unwrap()
                    .timeline
                    .tracks
                    .iter()
                    .flat_map(|track| track.clips.iter())
                    .all(|clip| !clip.gain_handle)
            }),
            "gain line is not drawn when here is Scene"
        );
        let _ = ui.bounds("inspector.utterance");
    }

    #[gpui::test]
    fn session_strip_has_no_locus_taking_buttons(cx: &mut TestAppContext) {
        let (view, cx) = add_studio(cx, overlap_session());
        let mut ui = UiDriver::new(cx);
        let _ = ui.bounds("toolbar.play");
        let _ = ui.bounds("toolbar.undo");
        let _ = ui.bounds("toolbar.resolve");
        let _ = ui.bounds("toolbar.delete-clip");
        let _ = ui.bounds("toolbar.cluster.clip");
        for gone in [
            "toolbar.set-in",
            "toolbar.set-out",
            "toolbar.split",
            "toolbar.gain-minus-3",
            "toolbar.fade",
        ] {
            assert!(
                ui.context().debug_bounds(gone).is_none(),
                "{gone} must not be drawn"
            );
        }
        let _ = ui.bounds("toolbar.cluster.file");
        let _ = ui.bounds("toolbar.cluster.clock");
        let _ = ui.bounds("toolbar.cluster.engine");
        let _ = ui.bounds("toolbar.cluster.clip");
        let _ = ui.bounds("toolbar.cluster.telemetry");
        for gone in [
            "inspector.split",
            "inspector.fade-out",
            "inspector.in-point",
            "inspector.out-point",
            "inspector.fade-out-field",
        ] {
            assert!(
                ui.context().debug_bounds(gone).is_none(),
                "{gone} must not be drawn"
            );
        }
        let _ = view;
    }

    #[gpui::test]
    fn inspector_typed_fields_and_invoked_record(cx: &mut TestAppContext) {
        let session = overlap_session();
        let video_id = session
            .layout()
            .unwrap()
            .timeline
            .tracks
            .iter()
            .find(|track| track.name == "Video")
            .unwrap()
            .clips[0]
            .id
            .clone();
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);
        ui.click(format!("timeline.clip.{video_id}"));
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session.current_locus().unwrap().unwrap().kind
            }),
            lattice_engine::LocusKind::Source
        );
        let _ = ui.bounds("inspector.gain");
        let _ = ui.bounds("inspector.fade");
        assert!(
            !ui.read(&view, |view, _| {
                view.session.layout().unwrap().inspector.title_fields
            }),
            "Title fields stay Title-only"
        );
        let original = ui.read(&view, |view, _| view.session.source().to_string());
        view.update(ui.context(), |view, cx| {
            view.gain_draft = "-6".into();
            view.commit_inspector_gain();
            cx.notify();
        });
        let after = ui.read(&view, |view, _| view.session.source().to_string());
        assert_ne!(after, original, "typed gain field must commit SetGain");
        assert!(!after.contains("by --"));
        let _ = ui.bounds("inspector.invoked");
        let invoked = ui.read(&view, |view, _| view.session.invoked_this_session());
        assert_eq!(invoked.len(), 1);
        assert_eq!(invoked[0].verb, "set-gain");
        view.update(ui.context(), |view, _| {
            view.gain_draft = "99".into();
        });
        let scene_id = ui.read(&view, |view, _| {
            view.session
                .layout()
                .unwrap()
                .timeline
                .tracks
                .iter()
                .find(|track| track.name == "Scene")
                .unwrap()
                .clips[0]
                .id
                .clone()
        });
        ui.click(format!("timeline.scene.{scene_id}"));
        let (kind, draft, fields) = ui.read(&view, |view, _| {
            (
                view.session.current_locus().unwrap().unwrap().kind,
                view.gain_draft.clone(),
                view.session.layout().unwrap().inspector.gain_db,
            )
        });
        assert_eq!(kind, lattice_engine::LocusKind::Scene);
        assert!(
            fields.is_none(),
            "gain field is bound to the source LocusId"
        );
        assert_ne!(
            draft, "99",
            "locus change must invalidate the in-flight draft"
        );
        let _ = view;
    }

    #[gpui::test]
    fn utterance_eye_does_not_apply_edit(cx: &mut TestAppContext) {
        let session = overlap_session();
        let video_id = session
            .layout()
            .unwrap()
            .timeline
            .tracks
            .iter()
            .find(|track| track.name == "Video")
            .unwrap()
            .clips[0]
            .id
            .clone();
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);
        ui.click(format!("timeline.clip.{video_id}"));
        view.update(ui.context(), |view, cx| {
            view.session.seek(lattice_engine::Time::seconds(5));
            cx.notify();
        });
        let (here, source) = ui.read(&view, |view, _| {
            (
                view.session.current_locus().unwrap().unwrap().id,
                view.session.source().to_string(),
            )
        });
        let _ = ui.bounds("utterance.eye");
        ui.click("utterance.eye");
        let (after_here, after_source, invoked) = ui.read(&view, |view, _| {
            (
                view.session.current_locus().unwrap().unwrap().id,
                view.session.source().to_string(),
                view.session.invoked_this_session(),
            )
        });
        assert_eq!(after_here, here, "eye must not retarget");
        assert_eq!(after_source, source, "eye must not apply_edit");
        assert!(invoked.is_empty(), "utterance never apply_edit");
        let _ = view;
    }

    #[gpui::test]
    fn on_target_handles_commit_gain_fade_split(cx: &mut TestAppContext) {
        let session = overlap_session();
        let video_id = session
            .layout()
            .unwrap()
            .timeline
            .tracks
            .iter()
            .find(|track| track.name == "Video")
            .unwrap()
            .clips[0]
            .id
            .clone();
        let audio_id = session
            .layout()
            .unwrap()
            .timeline
            .tracks
            .iter()
            .find(|track| track.name == "Audio")
            .unwrap()
            .clips[0]
            .id
            .clone();
        let scene_id = session
            .loci()
            .unwrap()
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Scene)
            .unwrap()
            .id
            .as_str()
            .to_string();
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);
        ui.click(format!("timeline.clip.{video_id}"));
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session.current_locus().unwrap().unwrap().kind
            }),
            lattice_engine::LocusKind::Source
        );
        let original = ui.read(&view, |view, _| view.session.source().to_string());
        ui.drag_within(
            format!("timeline.clip.{audio_id}"),
            (0.15, 0.9),
            (0.85, 0.9),
        );
        let after_body = ui.read(&view, |view, _| view.session.source().to_string());
        assert_eq!(
            after_body, original,
            "audio-block body must not commit SetGain"
        );
        ui.drag(
            format!("timeline.gain.{audio_id}"),
            (0.5, 0.5),
            format!("timeline.clip.{audio_id}"),
            (0.5, 0.05),
        );
        let after_gain = ui.read(&view, |view, _| view.session.source().to_string());
        assert_ne!(after_gain, original, "gain line must commit SetGain");
        assert!(!after_gain.contains("by --"));
        ui.drag_within(format!("timeline.fade.{video_id}"), (0.2, 0.5), (0.8, 0.5));
        let after_fade = ui.read(&view, |view, _| view.session.source().to_string());
        assert_ne!(after_fade, after_gain, "fade wedge must commit SetFade");

        ui.click(format!("timeline.scene.{scene_id}"));
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session.current_locus().unwrap().unwrap().kind
            }),
            lattice_engine::LocusKind::Scene
        );
        ui.click_at(format!("timeline.cut.{scene_id}"), 0.4, 0.5);
        let after_split = ui.read(&view, |view, _| view.session.source().to_string());
        assert_ne!(after_split, after_fade, "cut lane must commit split");
        let spoken = ui.read(&view, |view, _| view.session.utterance().spoken_text());
        assert!(!spoken.is_empty());
    }

    #[gpui::test]
    fn toolbar_delete_selected_clip_and_undo_restores(cx: &mut TestAppContext) {
        let session = overlap_session();
        let clip_id = session
            .layout()
            .unwrap()
            .timeline
            .tracks
            .iter()
            .find(|track| track.name == "Video")
            .unwrap()
            .clips[0]
            .id
            .clone();
        let original = session.source().to_string();
        let (view, cx) = add_studio(cx, session);
        let mut ui = UiDriver::new(cx);

        ui.click(format!("timeline.clip.{clip_id}"));
        assert_eq!(
            ui.read(&view, |view, _| {
                view.session
                    .current_locus()
                    .unwrap()
                    .map(|locus| locus.kind)
            }),
            Some(lattice_engine::LocusKind::Source)
        );
        let _ = ui.bounds("toolbar.delete-clip");
        assert!(
            ui.read(&view, |view, _| view.session.toolbar_shows_delete()),
            "toolbar Delete must be available for the selected clip"
        );

        ui.click("toolbar.delete-clip");
        let deleted = ui.read(&view, |view, _| view.session.source().to_string());
        assert_ne!(deleted, original, "toolbar Delete must rewrite source");
        assert!(
            !deleted.contains("as fight"),
            "selected source clip must be removed: {deleted}"
        );
        assert!(
            deleted.contains("title \"Hello\""),
            "unrelated title clip must remain: {deleted}"
        );

        ui.click("toolbar.undo");
        assert_eq!(
            ui.read(&view, |view, _| view.session.source().to_string()),
            original,
            "toolbar Undo must restore the deleted clip"
        );
    }
}
