//! Wasmtime host for the `lattice:stdlib` WIT world.
//!
//! No ambient network, filesystem, or random. Components emit Core semantics.

use lattice_core::{
    OverlayAlign, OverlayBar, OverlaySize, OverlayStyle, Placement, PlacementKind, Provenance,
    Rgba, Span as CoreSpan, Time, TimeMap as CoreTimeMap, TimeMapSegment as CoreSegment, Visual,
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

/// Hosted `freeze` / `title` / `caption` implementations.
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
            other => Err(LoweringError::Message(format!(
                "wasm stdlib does not implement `{other}`"
            ))),
        }
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
}
