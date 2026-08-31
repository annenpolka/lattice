//! Wasmtime host for the `lattice:stdlib` WIT world.
//!
//! No ambient network, filesystem, or random. Components emit Core semantics.

use lattice_core::{
    Audio, Media, MediaLocator, OverlayAlign, OverlayBar, OverlaySize, OverlayStyle, Placement,
    PlacementKind, Provenance, Rgba, Source, Span as CoreSpan, Time, TimeMap as CoreTimeMap,
    TimeMapSegment as CoreSegment, TimeSpan, Visual,
};

use crate::caption::merge_caption_timing;
use crate::overlay_body::{
    OverlayConvention, overlay_explain_notes, parse_overlay_body, parse_overlay_body_for,
};
use crate::view::{InvocationView, LoweringError, SceneDraft, ValueView};

wasmtime::component::bindgen!({
    path: "../../wit/lattice/stdlib.wit",
    world: "stdlib",
});

use exports::lattice::stdlib::lowering::{
    OverlayStyle as WitOverlayStyle, RationalTime, Span as WitSpan, TimeMap as WitTimeMap,
    TimeMapSegment as WitSegment,
};

const STDLIB_WASM: &[u8] = include_bytes!("../../../stdlib/lattice-stdlib.wasm");

struct Caps {
    ctx: wasmtime_wasi::WasiCtx,
    table: wasmtime::component::ResourceTable,
}

impl wasmtime_wasi::WasiView for Caps {
    fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
        wasmtime_wasi::WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

/// Hosted stdlib component. Invocation conversion and Core assembly stay here.
pub struct WasmStdlib {
    engine: wasmtime::Engine,
    component: wasmtime::component::Component,
    linker: wasmtime::component::Linker<Caps>,
}

impl WasmStdlib {
    pub fn load() -> Result<Self, LoweringError> {
        let mut config = wasmtime::Config::new();
        config.wasm_component_model(true);
        let engine = wasmtime::Engine::new(&config).map_err(map_err)?;
        let component =
            wasmtime::component::Component::from_binary(&engine, STDLIB_WASM).map_err(map_err)?;
        let mut linker = wasmtime::component::Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker).map_err(map_err)?;
        Ok(Self {
            engine,
            component,
            linker,
        })
    }

    pub fn overlay_presets(&self) -> Result<Vec<(String, OverlayStyle)>, LoweringError> {
        let (mut store, bindings) = self.instantiate()?;
        let entries = bindings
            .lattice_stdlib_lowering()
            .call_overlay_presets(&mut store)
            .map_err(map_err)?;
        entries
            .into_iter()
            .map(|(name, style)| Ok((name, from_wit_overlay_style(&style)?)))
            .collect()
    }

    pub fn lower(&self, inv: &InvocationView, draft: &mut SceneDraft) -> Result<(), LoweringError> {
        match inv.command.as_str() {
            "freeze" => self.lower_freeze(inv, draft),
            "title" => self.lower_title(inv, draft),
            "caption" => self.lower_caption(inv, draft),
            "callout" => self.lower_callout(inv, draft),
            "fade" => self.lower_fade(inv, draft),
            "gain" => self.lower_gain(inv, draft),
            "speech" => self.lower_speech(inv, draft),
            other => Err(LoweringError::Message(format!(
                "wasm stdlib does not implement `{other}`"
            ))),
        }
    }

    pub(crate) fn sequence_gap(&self, hold: Time) -> Result<Time, LoweringError> {
        let (mut store, bindings) = self.instantiate()?;
        let hold = bindings
            .lattice_stdlib_lowering()
            .call_gap(&mut store, to_wit_time(hold))
            .map_err(map_err)?
            .map_err(LoweringError::Message)?;
        from_wit_time(hold)
    }

    fn instantiate(&self) -> Result<(wasmtime::Store<Caps>, Stdlib), LoweringError> {
        let caps = Caps {
            ctx: wasmtime_wasi::WasiCtxBuilder::new().build(),
            table: wasmtime::component::ResourceTable::new(),
        };
        let mut store = wasmtime::Store::new(&self.engine, caps);
        let bindings =
            Stdlib::instantiate(&mut store, &self.component, &self.linker).map_err(map_err)?;
        Ok((store, bindings))
    }

    fn lower_freeze(
        &self,
        inv: &InvocationView,
        draft: &mut SceneDraft,
    ) -> Result<(), LoweringError> {
        let target = inv
            .args
            .first()
            .and_then(ValueView::as_name)
            .ok_or_else(|| LoweringError::Message("`freeze` needs a source name".into()))?;
        let at = inv
            .modifier("at")
            .and_then(ValueView::as_time)
            .ok_or_else(|| LoweringError::Message("`freeze` needs `at`".into()))?;
        let hold = inv
            .modifier("for")
            .and_then(ValueView::as_time)
            .ok_or_else(|| LoweringError::Message("`freeze` needs `for`".into()))?;
        let map = to_wit_map(&draft.source_mut(target)?.time_map.clone());
        let (mut store, bindings) = self.instantiate()?;
        let updated = bindings
            .lattice_stdlib_lowering()
            .call_freeze(&mut store, &map, to_wit_time(at), to_wit_time(hold))
            .map_err(map_err)?
            .map_err(LoweringError::Message)?;
        draft.source_mut(target)?.time_map = from_wit_map(&updated)?;
        draft.explain(
            lattice_core::Origin::Invocation {
                command: "freeze".into(),
            },
            format!("freeze `{target}` at {at} for {hold} -> TimeMap hold (rate 0)"),
        );
        Ok(())
    }

    fn lower_title(
        &self,
        inv: &InvocationView,
        draft: &mut SceneDraft,
    ) -> Result<(), LoweringError> {
        let text = inv
            .args
            .first()
            .and_then(ValueView::as_string)
            .ok_or_else(|| LoweringError::Message("`title` needs a string".into()))?
            .to_string();
        let at = inv
            .modifier("at")
            .and_then(ValueView::as_time)
            .unwrap_or(Time::ZERO);
        let hold = inv
            .modifier("for")
            .and_then(ValueView::as_time)
            .unwrap_or(Time::seconds(3));
        let opacity = title_opacity(inv);
        let parsed = parse_overlay_body(inv, draft);
        let span = to_wit_span(inv.span);
        let (mut store, bindings) = self.instantiate()?;
        let fragment = bindings
            .lattice_stdlib_lowering()
            .call_title(
                &mut store,
                &text,
                to_wit_time(at),
                to_wit_time(hold),
                opacity,
                span,
            )
            .map_err(map_err)?
            .map_err(LoweringError::Message)?;
        let at = from_wit_time(fragment.start)?;
        let hold = from_wit_time(fragment.duration)?;
        let id = draft.next_placement_id("title");
        let mut visual = Visual::text_overlay(fragment.text.clone());
        visual.opacity = fragment.opacity;
        visual.position = parsed.position;
        visual.scale = parsed.scale;
        visual.anchor = parsed.anchor;
        visual.style = parsed.style.clone().into_option();
        draft.placements.push(Placement {
            id,
            kind: PlacementKind::Title,
            source_id: None,
            span: lattice_core::TimeSpan::new(at, hold),
            visual: Some(visual),
            audio: None,
            provenance: Provenance::invocation("title", Some(inv.span)),
        });
        let opacity_note = fragment
            .opacity
            .map_or(String::new(), |value| format!(" opacity {value}"));
        let preset_note = crate::overlay_preset::preset_explain_note(
            parsed.applied_preset.as_deref(),
            parsed.applied_preset_source,
        );
        let style_notes = overlay_explain_notes(
            parsed.position,
            parsed.scale,
            parsed.anchor,
            &parsed.style,
            OverlayConvention::Title,
        );
        draft.explain(
            lattice_core::Origin::Invocation {
                command: "title".into(),
            },
            format!(
                "title {:?} at {at} for {hold}{opacity_note}{preset_note}{style_notes}",
                fragment.text
            ),
        );
        Ok(())
    }

    fn lower_caption(
        &self,
        inv: &InvocationView,
        draft: &mut SceneDraft,
    ) -> Result<(), LoweringError> {
        let text = inv
            .args
            .first()
            .and_then(ValueView::as_string)
            .ok_or_else(|| LoweringError::Message("`caption` needs a string".into()))?
            .to_string();
        let Some(timing) = merge_caption_timing(inv, draft) else {
            return Ok(());
        };
        let opacity = title_opacity(inv);
        let parsed = parse_overlay_body_for(inv, draft, OverlayConvention::Caption);
        let span = to_wit_span(inv.span);
        let (mut store, bindings) = self.instantiate()?;
        let fragment = bindings
            .lattice_stdlib_lowering()
            .call_caption(
                &mut store,
                &text,
                to_wit_time(timing.at),
                to_wit_time(timing.hold),
                opacity,
                span,
            )
            .map_err(map_err)?
            .map_err(LoweringError::Message)?;
        let at = from_wit_time(fragment.start)?;
        let hold = from_wit_time(fragment.duration)?;
        let id = draft.next_placement_id("caption");
        let mut visual = Visual::text_overlay(fragment.text.clone());
        visual.opacity = fragment.opacity;
        visual.position = parsed.position;
        visual.scale = parsed.scale;
        visual.anchor = parsed.anchor;
        visual.style = parsed.style.clone().into_option();
        draft.placements.push(Placement {
            id,
            kind: PlacementKind::Title,
            source_id: None,
            span: lattice_core::TimeSpan::new(at, hold),
            visual: Some(visual),
            audio: None,
            provenance: Provenance::invocation("caption", Some(inv.span)),
        });
        let opacity_note = fragment
            .opacity
            .map_or(String::new(), |value| format!(" opacity {value}"));
        let style_notes = overlay_explain_notes(
            parsed.position,
            parsed.scale,
            parsed.anchor,
            &parsed.style,
            OverlayConvention::Caption,
        );
        draft.explain(
            lattice_core::Origin::Invocation {
                command: "caption".into(),
            },
            format!(
                "caption {:?} at {at} for {hold}{opacity_note}{style_notes}",
                fragment.text
            ),
        );
        Ok(())
    }

    fn lower_callout(
        &self,
        inv: &InvocationView,
        draft: &mut SceneDraft,
    ) -> Result<(), LoweringError> {
        let text = inv
            .args
            .first()
            .and_then(ValueView::as_string)
            .ok_or_else(|| LoweringError::Message("`callout` needs a string".into()))?
            .to_string();
        let at = inv
            .modifier("at")
            .and_then(ValueView::as_time)
            .unwrap_or(Time::ZERO);
        let hold = inv
            .modifier("for")
            .and_then(ValueView::as_time)
            .unwrap_or(Time::seconds(2));
        let parsed = parse_overlay_body_for(inv, draft, OverlayConvention::Callout);
        let (mut store, bindings) = self.instantiate()?;
        let fragment = bindings
            .lattice_stdlib_lowering()
            .call_callout(
                &mut store,
                &text,
                to_wit_time(at),
                to_wit_time(hold),
                to_wit_span(inv.span),
            )
            .map_err(map_err)?
            .map_err(LoweringError::Message)?;
        let at = from_wit_time(fragment.start)?;
        let hold = from_wit_time(fragment.duration)?;
        let id = draft.next_placement_id("callout");
        let mut visual = Visual::text_overlay(fragment.text.clone());
        visual.position = parsed.position;
        visual.scale = parsed.scale;
        visual.anchor = parsed.anchor;
        visual.style = parsed.style.clone().into_option();
        draft.placements.push(Placement {
            id,
            kind: PlacementKind::Callout,
            source_id: None,
            span: TimeSpan::new(at, hold),
            visual: Some(visual),
            audio: None,
            provenance: Provenance::invocation("callout", Some(inv.span)),
        });
        let style_notes = overlay_explain_notes(
            parsed.position,
            parsed.scale,
            parsed.anchor,
            &parsed.style,
            OverlayConvention::Callout,
        );
        draft.explain(
            lattice_core::Origin::Invocation {
                command: "callout".into(),
            },
            format!(
                "callout {:?} at {at} for {hold}{style_notes}",
                fragment.text
            ),
        );
        Ok(())
    }

    fn lower_fade(
        &self,
        inv: &InvocationView,
        draft: &mut SceneDraft,
    ) -> Result<(), LoweringError> {
        let target = inv
            .args
            .first()
            .and_then(ValueView::as_name)
            .ok_or_else(|| LoweringError::Message("`fade` needs a source name".into()))?;
        let hold = inv
            .modifier("for")
            .and_then(ValueView::as_time)
            .ok_or_else(|| LoweringError::Message("`fade` needs `for`".into()))?;
        draft.source_mut(target)?;
        let (mut store, bindings) = self.instantiate()?;
        let hold = bindings
            .lattice_stdlib_lowering()
            .call_fade(&mut store, to_wit_time(hold))
            .map_err(map_err)?
            .map_err(LoweringError::Message)?;
        let hold = from_wit_time(hold)?;
        draft.source_fade_in.push((target.to_string(), hold));
        draft.explain(
            lattice_core::Origin::Invocation {
                command: "fade".into(),
            },
            format!("fade `{target}` in over {hold} (opacity envelope on video placement)"),
        );
        Ok(())
    }

    fn lower_gain(
        &self,
        inv: &InvocationView,
        draft: &mut SceneDraft,
    ) -> Result<(), LoweringError> {
        let target = inv
            .args
            .first()
            .and_then(ValueView::as_name)
            .ok_or_else(|| LoweringError::Message("`gain` needs a source name".into()))?;
        let db = inv
            .modifier("by")
            .and_then(ValueView::as_int)
            .or_else(|| inv.args.get(1).and_then(ValueView::as_int))
            .ok_or_else(|| LoweringError::Message("`gain` needs `by` (dB)".into()))?;
        draft.source_mut(target)?;
        let (mut store, bindings) = self.instantiate()?;
        let db = bindings
            .lattice_stdlib_lowering()
            .call_gain(&mut store, db)
            .map_err(map_err)?
            .map_err(LoweringError::Message)?;
        draft.source_gain_db.push((target.to_string(), db));
        draft.explain(
            lattice_core::Origin::Invocation {
                command: "gain".into(),
            },
            format!("gain `{target}` {db} dB"),
        );
        Ok(())
    }

    fn lower_speech(
        &self,
        inv: &InvocationView,
        draft: &mut SceneDraft,
    ) -> Result<(), LoweringError> {
        let text = inv
            .args
            .first()
            .and_then(ValueView::as_string)
            .ok_or_else(|| LoweringError::Message("`speech` needs a string".into()))?
            .to_string();
        let at = inv
            .modifier("at")
            .and_then(ValueView::as_time)
            .unwrap_or(Time::ZERO);
        let hold = inv
            .modifier("for")
            .and_then(ValueView::as_time)
            .unwrap_or(Time::seconds(2));
        let (mut store, bindings) = self.instantiate()?;
        let fragment = bindings
            .lattice_stdlib_lowering()
            .call_speech(
                &mut store,
                &text,
                to_wit_time(at),
                to_wit_time(hold),
                to_wit_span(inv.span),
            )
            .map_err(map_err)?
            .map_err(LoweringError::Message)?;
        let at = from_wit_time(fragment.start)?;
        let hold = from_wit_time(fragment.duration)?;
        let media_name = fragment.media_name;
        draft.media.push(Media {
            id: format!("media:{media_name}"),
            name: media_name.clone(),
            locator: MediaLocator::Generated {
                generator: "speech".into(),
                key: fragment.text.clone(),
            },
        });
        let source_id = format!("source:{media_name}");
        draft.sources.push(Source {
            id: source_id.clone(),
            name: media_name.clone(),
            media_name: media_name.clone(),
            source_range: TimeSpan::new(Time::ZERO, hold),
            time_map: CoreTimeMap::identity(Time::ZERO, hold),
            provenance: Provenance::invocation("speech", Some(inv.span)),
            generated: true,
        });
        let id = draft.next_placement_id("speech");
        draft.placements.push(Placement {
            id,
            kind: PlacementKind::Audio,
            source_id: Some(source_id),
            span: TimeSpan::new(at, hold),
            visual: None,
            audio: Some(Audio { gain_db: None }),
            provenance: Provenance::invocation("speech", Some(inv.span)),
        });
        draft.explain(
            lattice_core::Origin::Invocation {
                command: "speech".into(),
            },
            format!(
                "speech {:?} at {at} for {hold} -> generated media `{media_name}` (Resolve materializes; Compile does not)",
                fragment.text
            ),
        );
        Ok(())
    }
}

fn title_opacity(inv: &InvocationView) -> Option<u8> {
    use crate::view::BodyItem;
    inv.body.iter().find_map(|item| match item {
        BodyItem::Invocation(inner) if inner.command == "opacity" => inner
            .args
            .first()
            .and_then(ValueView::as_int)
            .and_then(|value| u8::try_from(value).ok()),
        _ => None,
    })
}

fn to_wit_time(time: Time) -> RationalTime {
    RationalTime {
        num: time.num(),
        den: time.den(),
    }
}

fn from_wit_time(time: RationalTime) -> Result<Time, LoweringError> {
    Time::new(time.num, time.den).map_err(|err| LoweringError::Message(err.to_string()))
}

fn to_wit_span(span: CoreSpan) -> WitSpan {
    WitSpan {
        start: span.start,
        end: span.end,
        line: span.line,
        column: span.column,
    }
}

fn to_wit_map(map: &CoreTimeMap) -> WitTimeMap {
    WitTimeMap {
        duration: to_wit_time(map.duration),
        segments: map
            .segments
            .iter()
            .map(|segment| WitSegment {
                local_start: to_wit_time(segment.local_start),
                local_duration: to_wit_time(segment.local_duration),
                content_start: to_wit_time(segment.content_start),
                rate: to_wit_time(segment.rate),
            })
            .collect(),
    }
}

fn from_wit_map(map: &WitTimeMap) -> Result<CoreTimeMap, LoweringError> {
    let mut segments = Vec::new();
    for segment in &map.segments {
        segments.push(CoreSegment {
            local_start: from_wit_time(segment.local_start)?,
            local_duration: from_wit_time(segment.local_duration)?,
            content_start: from_wit_time(segment.content_start)?,
            rate: from_wit_time(segment.rate)?,
        });
    }
    Ok(CoreTimeMap {
        duration: from_wit_time(map.duration)?,
        segments,
    })
}

fn from_wit_overlay_style(style: &WitOverlayStyle) -> Result<OverlayStyle, LoweringError> {
    let color = style
        .color
        .as_deref()
        .map(|hex| {
            Rgba::from_hex_rrggbb(hex).ok_or_else(|| {
                LoweringError::Message(format!("invalid overlay-preset color `{hex}`"))
            })
        })
        .transpose()?;
    let size = match (style.size_milli, style.size_px) {
        (Some(milli), None) => Some(OverlaySize::Percent { milli }),
        (None, Some(px)) => Some(OverlaySize::Px { px }),
        (None, None) => None,
        (Some(_), Some(_)) => {
            return Err(LoweringError::Message(
                "overlay-preset size cannot set both milli and px".into(),
            ));
        }
    };
    let bar = match style.bar.as_deref() {
        None => None,
        Some("off") => Some(OverlayBar::Off),
        Some(hex) => {
            let color = Rgba::from_hex_rrggbb(hex).ok_or_else(|| {
                LoweringError::Message(format!("invalid overlay-preset bar `{hex}`"))
            })?;
            Some(OverlayBar::Fill { color })
        }
    };
    let align = match style.align.as_deref() {
        None => None,
        Some("left") => Some(OverlayAlign::Left),
        Some("center") => Some(OverlayAlign::Center),
        Some("right") => Some(OverlayAlign::Right),
        Some(other) => {
            return Err(LoweringError::Message(format!(
                "invalid overlay-preset align `{other}`"
            )));
        }
    };
    Ok(OverlayStyle {
        color,
        size,
        weight: style.weight,
        family: style.family.clone(),
        bar,
        align,
    })
}

fn map_err(err: impl std::fmt::Display) -> LoweringError {
    LoweringError::Message(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::ValueView;

    fn empty_wit() -> WitOverlayStyle {
        WitOverlayStyle {
            color: None,
            size_milli: None,
            size_px: None,
            weight: None,
            family: None,
            bar: None,
            align: None,
        }
    }

    #[test]
    fn from_wit_maps_color_size_px_weight_align_bar_off() {
        let style = from_wit_overlay_style(&WitOverlayStyle {
            color: Some("#00FF00".into()),
            size_milli: None,
            size_px: Some(24),
            weight: Some(700),
            family: Some("GuestSans".into()),
            bar: Some("off".into()),
            align: Some("center".into()),
        })
        .unwrap();
        assert_eq!(style.color, Rgba::from_hex_rrggbb("#00FF00"));
        assert_eq!(style.size, Some(OverlaySize::Px { px: 24 }));
        assert_eq!(style.weight, Some(700));
        assert_eq!(style.family.as_deref(), Some("GuestSans"));
        assert_eq!(style.bar, Some(OverlayBar::Off));
        assert_eq!(style.align, Some(OverlayAlign::Center));
    }

    #[test]
    fn from_wit_maps_size_milli_and_bar_fill() {
        let style = from_wit_overlay_style(&WitOverlayStyle {
            size_milli: Some(900),
            bar: Some("#FFFF00".into()),
            align: Some("left".into()),
            ..empty_wit()
        })
        .unwrap();
        assert_eq!(style.size, Some(OverlaySize::Percent { milli: 900 }));
        assert_eq!(
            style.bar,
            Some(OverlayBar::Fill {
                color: Rgba::YELLOW
            })
        );
        assert_eq!(style.align, Some(OverlayAlign::Left));
    }

    #[test]
    fn from_wit_rejects_milli_and_px_together() {
        let err = from_wit_overlay_style(&WitOverlayStyle {
            size_milli: Some(900),
            size_px: Some(24),
            ..empty_wit()
        })
        .unwrap_err();
        assert!(err.to_string().contains("both milli and px"), "{err}");
    }

    #[test]
    fn from_wit_rejects_invalid_color_and_bar_hex() {
        let color = from_wit_overlay_style(&WitOverlayStyle {
            color: Some("green".into()),
            ..empty_wit()
        })
        .unwrap_err();
        assert!(color.to_string().contains("color"), "{color}");
        let bar = from_wit_overlay_style(&WitOverlayStyle {
            bar: Some("yellow".into()),
            ..empty_wit()
        })
        .unwrap_err();
        assert!(bar.to_string().contains("bar"), "{bar}");
    }

    #[test]
    fn from_wit_rejects_invalid_align() {
        let err = from_wit_overlay_style(&WitOverlayStyle {
            align: Some("middle".into()),
            ..empty_wit()
        })
        .unwrap_err();
        assert!(err.to_string().contains("align"), "{err}");
    }

    #[test]
    fn from_wit_maps_align_right() {
        let style = from_wit_overlay_style(&WitOverlayStyle {
            align: Some("right".into()),
            ..empty_wit()
        })
        .unwrap();
        assert_eq!(style.align, Some(OverlayAlign::Right));
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn hosted_component_lowers_callout_fade_gain_and_speech() {
        let wasm = WasmStdlib::load().unwrap();
        let span = CoreSpan::new(0, 1, 1, 1);
        let mut draft = SceneDraft {
            name: "demo".into(),
            sources: vec![Source {
                id: "source:clip".into(),
                name: "clip".into(),
                media_name: "game".into(),
                source_range: TimeSpan::new(Time::ZERO, Time::seconds(10)),
                time_map: CoreTimeMap::identity(Time::ZERO, Time::seconds(10)),
                provenance: Provenance::source(span),
                generated: false,
            }],
            ..SceneDraft::default()
        };
        let invocation = |command: &str,
                          args: Vec<ValueView>,
                          modifiers: Vec<(&str, ValueView)>|
         -> InvocationView {
            InvocationView {
                command: command.into(),
                args,
                modifiers: modifiers
                    .into_iter()
                    .map(|(name, value)| (name.into(), value))
                    .collect(),
                body: Vec::new(),
                span,
            }
        };

        wasm.lower(
            &invocation(
                "callout",
                vec![ValueView::String("Look".into())],
                vec![
                    ("at", ValueView::Time(Time::seconds(1))),
                    ("for", ValueView::Time(Time::seconds(2))),
                ],
            ),
            &mut draft,
        )
        .unwrap();
        wasm.lower(
            &invocation(
                "fade",
                vec![ValueView::Name("clip".into())],
                vec![("for", ValueView::Time(Time::milliseconds(500)))],
            ),
            &mut draft,
        )
        .unwrap();
        wasm.lower(
            &invocation(
                "gain",
                vec![ValueView::Name("clip".into())],
                vec![(
                    "by",
                    ValueView::Quantity {
                        negative: true,
                        digits: 6,
                        scale: 0,
                        unit: None,
                    },
                )],
            ),
            &mut draft,
        )
        .unwrap();
        wasm.lower(
            &invocation(
                "speech",
                vec![ValueView::String("Nice freeze".into())],
                vec![
                    ("at", ValueView::Time(Time::seconds(3))),
                    ("for", ValueView::Time(Time::seconds(2))),
                ],
            ),
            &mut draft,
        )
        .unwrap();

        assert_eq!(
            draft.source_fade_in[0],
            ("clip".into(), Time::milliseconds(500))
        );
        assert_eq!(draft.source_gain_db[0], ("clip".into(), -6));
        assert!(draft.placements.iter().any(|placement| {
            placement.kind == PlacementKind::Callout
                && placement
                    .visual
                    .as_ref()
                    .and_then(|visual| visual.text.as_deref())
                    == Some("Look")
        }));
        assert!(draft.media.iter().any(|media| {
            media.name == "speech-nice-freeze"
                && matches!(
                    &media.locator,
                    MediaLocator::Generated { generator, key }
                        if generator == "speech" && key == "Nice freeze"
                )
        }));
        assert!(draft.sources.iter().any(|source| source.generated));
    }
}
