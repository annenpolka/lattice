//! Evaluate a flattened timeline at time `t` into a `RenderScene` snapshot + `AudioPlan`.
#![allow(clippy::cast_possible_truncation)]

use crate::NormalizedScale;
use crate::ir::{PlacementKind, TimeSpan};
use crate::locator::MediaLocator;
use crate::overlay::{OverlayAlign, OverlayBar, OverlaySize, OverlayStyle};
use crate::property::{Curve, Easing, Keyframe, Property};
use crate::scene::{
    AnimatedStyle, AssetRef, AudioClip, AudioPlan, BlendMode, Canvas, FontIdentity, FontSpec,
    GroupNode, NodeProps, Rect, RenderNode, RenderScene, Rgba, ShapeKind, ShapeNode, TextNode,
    Transform, VideoNode,
};
use crate::time::Time;
use crate::timeline::{Timeline, TimelineClip, TimelineError};
