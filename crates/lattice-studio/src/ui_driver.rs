//! Selector-driven GPUI interaction driver for Studio tests.
//!
//! This intentionally dispatches platform input through [`gpui::VisualTestContext`]. It never
//! calls a view handler directly and never stores absolute screen coordinates in a test.

use gpui::{
    App, Bounds, Entity, Modifiers, MouseButton, Pixels, Point, ScrollDelta, ScrollWheelEvent,
    Size, VisualTestContext, point, px,
};

const DRAG_STEPS: usize = 5;

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
        control: bool,
    ) {
        let position = self.point_at(selector, 0.5, 0.5);
        self.cx.simulate_event(ScrollWheelEvent {
            position,
            delta: ScrollDelta::Pixels(point(px(delta_x), px(delta_y))),
            modifiers: Modifiers {
                control,
                ..Modifiers::none()
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

    use super::UiDriver;
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
        ui.press("ctrl-z");
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

        // Re-select the video scene so trim handles are rendered after scrub changed the locus.
        view.update(ui.context(), |view, cx| {
            view.session.point_at(LocusId::new("scene:intro"));
            cx.notify();
        });
        ui.context().run_until_parked();
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

        ui.press("ctrl-z");
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
}
