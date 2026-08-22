//! Generation-ordered, bounded preview scheduling for Studio.
//!
//! The playhead is paced by a monotonic wall clock and video frames are sampled at or before that
//! playhead. Studio Play waits for its `AudioPlan` PCM monitor, then keeps audio transport and video
//! sampling on the same session playhead. The inbox retains one pending job in addition to the
//! worker's active job, so a slow decoder cannot build an unbounded queue.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use lattice_engine::{Compilation, RawFrame, RendererRequest, Time};

/// UI cadence. Source-frame snapping below prevents duplicate decoder work at lower frame rates.
pub const PLAYBACK_TICK: Duration = Duration::from_millis(16);

/// Derive an absolute playhead from the playback origin instead of accumulating timer deltas.
/// This avoids drift when UI ticks are late or coalesced.
#[must_use]
pub fn playback_target(origin: Time, elapsed: Duration, duration: Time) -> Time {
    let micros = i64::try_from(elapsed.as_micros()).unwrap_or(i64::MAX);
    let elapsed = Time::new(micros, 1_000_000).unwrap_or(Time::ZERO);
    origin
        .checked_add(elapsed)
        .unwrap_or(duration)
        .max(Time::ZERO)
        .min(duration)
}

/// Frame at or immediately before `playhead`. Playback must never reveal a future source frame.
#[must_use]
pub fn playback_frame_at_or_before(playhead: Time, fps_num: i64, fps_den: i64) -> Time {
    if playhead <= Time::ZERO || fps_num <= 0 || fps_den <= 0 {
        return playhead.max(Time::ZERO);
    }
    let numerator = i128::from(playhead.num()).checked_mul(i128::from(fps_num));
    let denominator = i128::from(playhead.den()).checked_mul(i128::from(fps_den));
    let Some((numerator, denominator)) = numerator.zip(denominator) else {
        return playhead;
    };
    if denominator <= 0 {
        return playhead;
    }
    let frames = numerator
        .div_euclid(denominator)
        .clamp(0, i128::from(i64::MAX));
    i64::try_from(frames)
        .ok()
        .and_then(|frames| Time::from_frames(frames, fps_num, fps_den).ok())
        .unwrap_or(playhead)
}

/// Coalescing mailbox for async preview frames.
#[derive(Clone, Debug, Default)]
pub struct PreviewMailbox {
    stamp: String,
    current: u64,
    valid_from: u64,
    published: u64,
    published_path: Option<PathBuf>,
    current_frame: Option<Arc<RawFrame>>,
    previous_frame: Option<Arc<RawFrame>>,
    published_time: Option<Time>,
}

impl PreviewMailbox {
    #[must_use]
    pub fn current_generation(&self) -> u64 {
        self.current
    }

    #[must_use]
    pub fn published_generation(&self) -> u64 {
        self.published
    }

    #[must_use]
    pub fn published_time(&self) -> Option<Time> {
        self.published_time
    }

    #[must_use]
    pub fn published_path(&self) -> Option<&Path> {
        self.published_path.as_deref()
    }

    /// Latest in-memory frame. Studio's play path uses this instead of a still file.
    #[must_use]
    pub fn published_frame(&self) -> Option<&Arc<RawFrame>> {
        self.current_frame.as_ref()
    }

    /// The mailbox retains at most the displayed frame and its predecessor.
    #[must_use]
    pub fn retained_frame_count(&self) -> usize {
        usize::from(self.current_frame.is_some()) + usize::from(self.previous_frame.is_some())
    }

    pub fn set_stamp(&mut self, stamp: impl Into<String>) {
        self.stamp = stamp.into();
    }

    #[must_use]
    pub fn stamp(&self) -> &str {
        &self.stamp
    }

    /// Open a new request. Returns the generation id the worker must echo.
    pub fn request(&mut self) -> u64 {
        self.current = self.current.saturating_add(1);
        self.current
    }

    /// Publish `path` if `generation` is still the newest request.
    /// Returns whether the frame became the displayed result.
    pub fn accept(&mut self, generation: u64, path: PathBuf, time: Time) -> bool {
        self.accept_stamped(generation, path, time, "")
    }

    pub fn accept_stamped(
        &mut self,
        generation: u64,
        path: PathBuf,
        time: Time,
        stamp: &str,
    ) -> bool {
        if !self.stamp.is_empty() && stamp != self.stamp {
            return false;
        }
        if generation < self.valid_from {
            return false;
        }
        if generation != self.current {
            return false;
        }
        if generation < self.published {
            return false;
        }
        self.published = generation;
        self.published_path = Some(path);
        self.published_time = Some(time);
        true
    }

    /// Publish an in-memory frame. While playing, a completed request may publish even when a
    /// newer request is queued; this prevents a slower decoder from starving forever. Paused
    /// scrub remains strict and only accepts the newest generation.
    pub fn accept_frame_stamped(
        &mut self,
        generation: u64,
        frame: Arc<RawFrame>,
        time: Time,
        stamp: &str,
        playing: bool,
    ) -> bool {
        if !self.stamp.is_empty() && stamp != self.stamp {
            return false;
        }
        if generation > self.current || generation < self.valid_from || generation < self.published
        {
            return false;
        }
        if !playing && generation != self.current {
            return false;
        }
        if playing
            && self
                .published_time
                .is_some_and(|published| time < published)
        {
            return false;
        }
        self.published = generation;
        self.previous_frame = self.current_frame.replace(frame);
        self.published_time = Some(time);
        true
    }

    /// Drop in-flight gens but keep the last still on screen until a newer one lands.
    pub fn invalidate(&mut self) {
        self.current = self.current.saturating_add(1);
        self.valid_from = self.current;
        self.published = 0;
        self.published_time = None;
    }

    pub fn clear(&mut self) {
        self.current = self.current.saturating_add(1);
        self.valid_from = self.current;
        self.published = 0;
        self.published_path = None;
        self.current_frame = None;
        self.previous_frame = None;
        self.published_time = None;
    }
}

/// Description of one off-UI-path extract. GPUI must not run this on paint.
#[derive(Clone, Debug)]
pub struct PreviewJob {
    pub generation: u64,
    pub timeline_time: Time,
    pub width: u32,
    pub height: u32,
    pub fps_num: i64,
    pub fps_den: i64,
    pub renderer: RendererRequest,
    pub output: PathBuf,
    pub media_root: PathBuf,
    pub compilation: Compilation,
    pub source_revision: String,
    pub stamp: String,
    pub lock_stamp: String,
}

/// Result of pushing into the bounded worker inbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreviewPush {
    Queued,
    Replaced,
    Stopped,
}

/// Observable scheduler state used by process diagnostics and deterministic tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PreviewInboxStats {
    pub pending: usize,
    pub in_flight: usize,
    pub replaced_pending: u64,
    pub stopped: bool,
}

/// Latest-only inbox for the sample worker. Capacity is one pending plus one in-flight job.
pub struct PreviewInbox {
    state: Mutex<InboxState>,
    cv: Condvar,
}

struct InboxState {
    job: Option<PreviewJob>,
    active_generation: Option<u64>,
    replaced_pending: u64,
    reset_sampler_at: Option<u64>,
    stop: bool,
}

impl PreviewInbox {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(InboxState {
                job: None,
                active_generation: None,
                replaced_pending: 0,
                reset_sampler_at: None,
                stop: false,
            }),
            cv: Condvar::new(),
        })
    }

    pub fn push(&self, job: PreviewJob) -> PreviewPush {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.stop {
            return PreviewPush::Stopped;
        }
        let replaced = guard.job.is_some();
        if replaced {
            guard.replaced_pending = guard.replaced_pending.saturating_add(1);
        }
        guard.job = Some(job);
        self.cv.notify_one();
        if replaced {
            PreviewPush::Replaced
        } else {
            PreviewPush::Queued
        }
    }

    pub fn take_wait(&self) -> Option<PreviewJob> {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if guard.stop {
                return None;
            }
            if let Some(job) = guard.job.take() {
                guard.active_generation = Some(job.generation);
                return Some(job);
            }
            guard = self
                .cv
                .wait(guard)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }

    /// Mark the worker's active generation complete. A newer pending job remains untouched.
    pub fn complete(&self, generation: u64) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard.active_generation == Some(generation) {
            guard.active_generation = None;
        }
    }

    /// Discard queued work on a transport reset. The active decoder call is invalidated by the
    /// session generation and may finish safely in the background.
    pub fn clear_pending(&self) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.job = None;
    }

    /// Make the worker job at or after `generation` recreate its renderer-backed sampler.
    ///
    /// Studio uses this only after an explicit renderer selection/retry. It is deliberately
    /// separate from ordinary invalidation so edits, seeks, and playback ticks cannot silently
    /// recover by changing or recreating a failed backend.
    pub fn request_sampler_reset(&self, generation: u64) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.reset_sampler_at = Some(generation);
    }

    /// Consume the one-shot sampler reset immediately before the matching worker job.
    pub fn take_sampler_reset(&self, generation: u64) -> bool {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if guard
            .reset_sampler_at
            .is_some_and(|target| generation >= target)
        {
            guard.reset_sampler_at = None;
            true
        } else {
            false
        }
    }

    #[must_use]
    pub fn stats(&self) -> PreviewInboxStats {
        let guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        PreviewInboxStats {
            pending: usize::from(guard.job.is_some()),
            in_flight: usize::from(guard.active_generation.is_some()),
            replaced_pending: guard.replaced_pending,
            stopped: guard.stop,
        }
    }

    pub fn stop(&self) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.stop = true;
        guard.job = None;
        guard.reset_sampler_at = None;
        self.cv.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_clock_target_does_not_accumulate_tick_drift() {
        let duration = Time::seconds(10);
        assert_eq!(
            playback_target(Time::seconds(2), Duration::from_millis(1_250), duration),
            Time::from_decimal_seconds(3, 25, 2).unwrap()
        );
        assert_eq!(
            playback_target(Time::seconds(9), Duration::from_secs(5), duration),
            duration
        );
    }

    #[test]
    fn playback_sampling_never_rounds_into_the_future() {
        assert_eq!(
            playback_frame_at_or_before(Time::milliseconds(99), 10, 1),
            Time::ZERO
        );
        assert_eq!(
            playback_frame_at_or_before(Time::milliseconds(149), 10, 1),
            Time::milliseconds(100)
        );
        assert_eq!(
            playback_frame_at_or_before(Time::milliseconds(34), 30_000, 1_001),
            Time::from_frames(1, 30_000, 1_001).unwrap()
        );
    }

    #[test]
    fn invalidation_is_a_hard_epoch_even_after_play_resumes() {
        let mut mailbox = PreviewMailbox::default();
        mailbox.set_stamp("project-a");
        let old = mailbox.request();
        mailbox.invalidate();
        let current = mailbox.request();
        let frame = Arc::new(RawFrame::filled(2, 2, 0, 0, 0, 255));
        assert!(!mailbox.accept_frame_stamped(
            old,
            Arc::clone(&frame),
            Time::seconds(2),
            "project-a",
            true,
        ));
        assert!(mailbox.accept_frame_stamped(current, frame, Time::seconds(1), "project-a", true,));
    }

    #[test]
    fn playing_frames_are_monotonic_but_may_finish_behind_latest_request() {
        let mut mailbox = PreviewMailbox::default();
        mailbox.set_stamp("project-a");
        let slow = mailbox.request();
        let newest = mailbox.request();
        let frame = Arc::new(RawFrame::filled(2, 2, 0, 0, 0, 255));
        assert!(mailbox.accept_frame_stamped(
            slow,
            Arc::clone(&frame),
            Time::seconds(1),
            "project-a",
            true,
        ));
        assert!(!mailbox.accept_frame_stamped(
            newest,
            Arc::clone(&frame),
            Time::milliseconds(900),
            "project-a",
            true,
        ));
        assert!(mailbox.accept_frame_stamped(newest, frame, Time::seconds(2), "project-a", true,));
        assert_eq!(mailbox.published_time(), Some(Time::seconds(2)));
    }

    #[test]
    fn sampler_reset_is_explicit_and_one_shot() {
        let inbox = PreviewInbox::new();
        assert!(!inbox.take_sampler_reset(7));
        inbox.request_sampler_reset(7);
        assert!(!inbox.take_sampler_reset(6));
        assert!(inbox.take_sampler_reset(7));
        assert!(!inbox.take_sampler_reset(8));
    }
}
