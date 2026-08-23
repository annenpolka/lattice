//! Shaping / layout / glyph raster via cosmic-text. `FFmpeg` `drawtext` is not used.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss
)]

use std::collections::HashMap;

use cosmic_text::{
    Attrs, Buffer, Color, Family, FontSystem, Metrics, Shaping, Style, SwashCache, Weight, Wrap,
};

use lattice_core::{FontSpec, Rgba, TextNode};

use crate::backend::RawFrame;
use crate::export::ExportError;
use crate::font::FontResolution;

const TEXT_RASTER_SCHEMA: u16 = 1;
const TEXT_LOCALE: &str = "ja";
const LINE_HEIGHT_MILLI: u32 = 1_250;
const SHAPING_KEY: &str = "advanced";
const WRAP_KEY: &str = "word-or-glyph";

/// Bounds for retained node-local text rasters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextCacheLimits {
    pub max_entries: usize,
    pub max_bytes: usize,
}

impl Default for TextCacheLimits {
    fn default() -> Self {
        Self {
            max_entries: 128,
            max_bytes: 16 * 1024 * 1024,
        }
    }
}

/// Lifetime counters plus current resident RGBA storage.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: usize,
    pub bytes: usize,
}

/// Stable semantic identity for a shaped/rasterized text run.
///
/// Canvas position and compositing properties deliberately stay out: a local
/// layer can be reused while the GPU applies transform, opacity, clip, and
/// blend. The schema and shaping constants keep changes to raster semantics
/// observable instead of accidentally aliasing old entries.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextRasterKey {
    schema: u16,
    font_identity: String,
    face_index: u32,
    run: String,
    family: String,
    weight: u16,
    italic: bool,
    size_px: u32,
    width: u32,
    height: u32,
    color: [u8; 4],
    locale: &'static str,
    line_height_milli: u32,
    shaping: &'static str,
    wrap: &'static str,
}

impl TextRasterKey {
    fn new(font: &FontResolution, node: &TextNode) -> Self {
        Self {
            schema: TEXT_RASTER_SCHEMA,
            font_identity: font.identity.identity.as_str().to_owned(),
            face_index: font.identity.face_index,
            run: node.text.clone(),
            family: node.font.family.clone(),
            weight: node.font.weight,
            italic: node.font.italic,
            size_px: node.font.size_px,
            width: node.bounds.width,
            height: node.bounds.height,
            color: [node.color.r, node.color.g, node.color.b, node.color.a],
            locale: TEXT_LOCALE,
            line_height_milli: LINE_HEIGHT_MILLI,
            shaping: SHAPING_KEY,
            wrap: WRAP_KEY,
        }
    }

    /// Fixed-field FNV-1a fingerprint for diagnostics/tests. `HashMap` correctness
    /// still uses full key equality, so a fingerprint collision cannot alias.
    fn stable_fingerprint(&self) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325;
        stable_field(&mut hash, &self.schema.to_le_bytes());
        stable_field(&mut hash, self.font_identity.as_bytes());
        stable_field(&mut hash, &self.face_index.to_le_bytes());
        stable_field(&mut hash, self.run.as_bytes());
        stable_field(&mut hash, self.family.as_bytes());
        stable_field(&mut hash, &self.weight.to_le_bytes());
        stable_field(&mut hash, &[u8::from(self.italic)]);
        stable_field(&mut hash, &self.size_px.to_le_bytes());
        stable_field(&mut hash, &self.width.to_le_bytes());
        stable_field(&mut hash, &self.height.to_le_bytes());
        stable_field(&mut hash, &self.color);
        stable_field(&mut hash, self.locale.as_bytes());
        stable_field(&mut hash, &self.line_height_milli.to_le_bytes());
        stable_field(&mut hash, self.shaping.as_bytes());
        stable_field(&mut hash, self.wrap.as_bytes());
        hash
    }
}

fn stable_field(hash: &mut u64, bytes: &[u8]) {
    for byte in (bytes.len() as u64).to_le_bytes().iter().chain(bytes) {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100_0000_01b3);
    }
}

pub struct TextRasterizer {
    font_system: FontSystem,
    glyph_cache: SwashCache,
}

impl TextRasterizer {
    pub fn new(font: &FontResolution) -> Self {
        let mut db = cosmic_text::fontdb::Database::new();
        db.load_font_data(font.bytes.clone());
        db.set_sans_serif_family("M PLUS 1p");
        let font_system = FontSystem::new_with_locale_and_db(TEXT_LOCALE.into(), db);
        Self {
            font_system,
            glyph_cache: SwashCache::new(),
        }
    }

    /// Rasterize only this node's layout extent, at a local `(0, 0)` origin.
    pub fn rasterize_layer(&mut self, node: &TextNode) -> Result<RawFrame, ExportError> {
        let mut layer = RawFrame::filled(
            node.bounds.width.max(1),
            node.bounds.height.max(1),
            0,
            0,
            0,
            0,
        );
        self.draw_into_layer(&mut layer, node)?;
        Ok(layer)
    }

    fn draw_into_layer(
        &mut self,
        frame: &mut RawFrame,
        node: &TextNode,
    ) -> Result<(), ExportError> {
        if node.text.is_empty() {
            return Ok(());
        }
        let size = node.font.size_px.max(8) as f32;
        let metrics = Metrics::new(size, size * LINE_HEIGHT_MILLI as f32 / 1_000.0);
        let mut buffer = Buffer::new(&mut self.font_system, metrics);
        let width = node.bounds.width as f32;
        let height = node.bounds.height as f32;
        buffer.set_size(
            &mut self.font_system,
            Some(width.max(1.0)),
            Some(height.max(1.0)),
        );
        buffer.set_wrap(&mut self.font_system, Wrap::WordOrGlyph);
        let attrs = attrs_for(&node.font);
        buffer.set_text(&mut self.font_system, &node.text, &attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut self.font_system, false);
        if let Some(missing) = missing_glyph_excerpt(&buffer, &node.text) {
            return Err(ExportError::MissingGlyph(missing));
        }
        let color = Color::rgba(node.color.r, node.color.g, node.color.b, node.color.a);
        buffer.draw(
            &mut self.font_system,
            &mut self.glyph_cache,
            color,
            |x, y, w, h, glyph_color| {
                blit_coverage(frame, x, y, w, h, glyph_color, 100, node.color);
            },
        );
        Ok(())
    }
}

struct TextCacheEntry {
    frame: RawFrame,
    last_used: u64,
}

/// Bounded LRU for node-local transparent rasters. The byte budget counts
/// retained packed RGBA bytes; entry count independently bounds key overhead.
pub(crate) struct TextLayerCache {
    rasterizer: TextRasterizer,
    font: FontResolution,
    limits: TextCacheLimits,
    stats: TextCacheStats,
    clock: u64,
    entries: HashMap<TextRasterKey, TextCacheEntry>,
}

impl TextLayerCache {
    pub(crate) fn new(font: &FontResolution, limits: TextCacheLimits) -> Self {
        Self {
            rasterizer: TextRasterizer::new(font),
            font: font.clone(),
            limits,
            stats: TextCacheStats::default(),
            clock: 0,
            entries: HashMap::new(),
        }
    }

    pub(crate) fn stats(&self) -> TextCacheStats {
        self.stats
    }

    pub(crate) fn get_or_rasterize(&mut self, node: &TextNode) -> Result<RawFrame, ExportError> {
        let key = TextRasterKey::new(&self.font, node);
        let now = self.next_clock();
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_used = now;
            self.stats.hits = self.stats.hits.saturating_add(1);
            return Ok(entry.frame.clone());
        }

        self.stats.misses = self.stats.misses.saturating_add(1);
        let frame = self.rasterizer.rasterize_layer(node)?;
        let bytes = frame.rgba.len();
        if self.limits.max_entries == 0 || bytes > self.limits.max_bytes {
            return Ok(frame);
        }
        while self.entries.len() >= self.limits.max_entries
            || self.stats.bytes.saturating_add(bytes) > self.limits.max_bytes
        {
            if !self.evict_lru() {
                break;
            }
        }
        self.entries.insert(
            key,
            TextCacheEntry {
                frame: frame.clone(),
                last_used: now,
            },
        );
        self.stats.entries = self.entries.len();
        self.stats.bytes = self.stats.bytes.saturating_add(bytes);
        Ok(frame)
    }

    fn evict_lru(&mut self) -> bool {
        let Some(key) = self
            .entries
            .iter()
            .min_by(|(key_a, entry_a), (key_b, entry_b)| {
                entry_a
                    .last_used
                    .cmp(&entry_b.last_used)
                    .then_with(|| key_a.stable_fingerprint().cmp(&key_b.stable_fingerprint()))
            })
            .map(|(key, _)| key.clone())
        else {
            return false;
        };
        let entry = self.entries.remove(&key).expect("LRU key came from map");
        self.stats.evictions = self.stats.evictions.saturating_add(1);
        self.stats.entries = self.entries.len();
        self.stats.bytes = self.stats.bytes.saturating_sub(entry.frame.rgba.len());
        true
    }

    fn next_clock(&mut self) -> u64 {
        if self.clock == u64::MAX {
            let mut ordered: Vec<_> = self.entries.values_mut().collect();
            ordered.sort_by_key(|entry| entry.last_used);
            for (index, entry) in ordered.into_iter().enumerate() {
                entry.last_used = index as u64;
            }
            self.clock = self.entries.len() as u64;
        }
        self.clock += 1;
        self.clock
    }
}

fn missing_glyph_excerpt(buffer: &Buffer, text: &str) -> Option<String> {
    let mut missing = String::new();
    for run in buffer.layout_runs() {
        for glyph in run.glyphs {
            if glyph.glyph_id != 0 {
                continue;
            }
            let slice = text.get(glyph.start..glyph.end).unwrap_or("?");
            if slice.chars().all(char::is_whitespace) {
                continue;
            }
            if !missing.is_empty() {
                missing.push(' ');
            }
            missing.push_str(slice);
            if missing.len() > 48 {
                break;
            }
        }
    }
    if missing.is_empty() {
        None
    } else {
        Some(missing)
    }
}

fn attrs_for(spec: &FontSpec) -> Attrs<'_> {
    Attrs::new()
        .family(Family::Name(&spec.family))
        .weight(Weight(spec.weight))
        .style(if spec.italic {
            Style::Italic
        } else {
            Style::Normal
        })
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments
)]
fn blit_coverage(
    frame: &mut RawFrame,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    glyph_color: Color,
    opacity: u8,
    fallback: Rgba,
) {
    let [gr, gg, gb, ga] = [
        glyph_color.r(),
        glyph_color.g(),
        glyph_color.b(),
        glyph_color.a(),
    ];
    let alpha = u16::from(ga) * u16::from(opacity) / 100;
    if alpha == 0 {
        return;
    }
    for dy in 0..h {
        for dx in 0..w {
            let px = x.saturating_add(dx as i32);
            let py = y.saturating_add(dy as i32);
            if px < 0 || py < 0 {
                continue;
            }
            src_over(
                frame,
                px as u32,
                py as u32,
                [
                    gr.max(fallback.r),
                    gg.max(fallback.g),
                    gb.max(fallback.b),
                    alpha as u8,
                ],
            );
        }
    }
}

fn src_over(frame: &mut RawFrame, x: u32, y: u32, src: [u8; 4]) {
    let Some(dst) = frame.pixel(x, y) else {
        return;
    };
    let a = u16::from(src[3]);
    if a == 0 {
        return;
    }
    let inv = 255 - a;
    let blend = |s: u8, d: u8| ((u16::from(s) * a + u16::from(d) * inv) / 255) as u8;
    frame.set_pixel(
        x,
        y,
        [
            blend(src[0], dst[0]),
            blend(src[1], dst[1]),
            blend(src[2], dst[2]),
            255,
        ],
    );
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use lattice_core::{BlendMode, FontSpec, NodeProps, Rect, Rgba, TextNode, Transform};

    use super::*;
    use crate::font::{fixture_font_path, resolve_font};

    fn fixture_font() -> FontResolution {
        let path = fixture_font_path().expect("repository fixture font");
        resolve_font(
            &FontSpec::preview_sans(18),
            Path::new("."),
            None,
            Some(&path),
        )
        .expect("resolve fixture font")
    }

    fn node(text: &str, width: u32, height: u32) -> TextNode {
        TextNode {
            props: NodeProps::opaque(1),
            bounds: Rect {
                x: 7,
                y: 11,
                width,
                height,
            },
            text: text.into(),
            font: FontSpec::preview_sans(18),
            resolved_font: None,
            color: Rgba::WHITE,
        }
    }

    #[test]
    fn attrs_honor_family_and_weight() {
        let spec = FontSpec {
            family: "LatticeSans".into(),
            weight: 700,
            italic: false,
            size_px: 18,
        };
        let attrs = attrs_for(&spec);
        assert!(matches!(attrs.family, Family::Name("LatticeSans")));
        assert_eq!(attrs.weight, Weight(700));
        let preview = FontSpec::preview_sans(18);
        let normal = attrs_for(&preview);
        assert!(matches!(normal.family, Family::Name("LatticeSans")));
        assert_eq!(normal.weight, Weight(400));
    }

    #[test]
    fn stable_key_separates_raster_inputs_from_gpu_compositing() {
        let font = fixture_font();
        let base = node("Title 日本語", 96, 28);
        let key = TextRasterKey::new(&font, &base);
        let mut composited = base.clone();
        composited.bounds.x = 101;
        composited.bounds.y = -5;
        composited.props = NodeProps {
            transform: Transform {
                translate_x: 4,
                translate_y: 2,
                scale_x: 1_500,
                scale_y: 1_500,
                rotation_mdeg: 90_000,
            },
            opacity: 42,
            clip: Some(Rect {
                x: 8,
                y: 9,
                width: 20,
                height: 10,
            }),
            z: 99,
            blend: BlendMode::Multiply,
        };
        let composited_key = TextRasterKey::new(&font, &composited);
        assert_eq!(key, composited_key);
        assert_eq!(
            key.stable_fingerprint(),
            composited_key.stable_fingerprint()
        );

        composited.bounds.width += 1;
        let resized = TextRasterKey::new(&font, &composited);
        assert_ne!(key, resized);
        assert_ne!(key.stable_fingerprint(), resized.stable_fingerprint());
        composited.bounds.width -= 1;
        composited.color = Rgba::CYAN;
        assert_ne!(key, TextRasterKey::new(&font, &composited));

        let mut different_run = base.clone();
        different_run.text.push('!');
        assert_ne!(key, TextRasterKey::new(&font, &different_run));
        let mut different_font = font.clone();
        different_font.identity.identity = lattice_core::AssetIdentity::new("other-font-content");
        assert_ne!(key, TextRasterKey::new(&different_font, &base));
    }

    #[test]
    fn entry_lru_is_observable_and_eviction_is_raw_frame_deterministic() {
        let font = fixture_font();
        let mut cache = TextLayerCache::new(
            &font,
            TextCacheLimits {
                max_entries: 1,
                max_bytes: 1024 * 1024,
            },
        );
        let title = node("Representative title", 128, 32);
        let callout = node("Representative callout", 128, 32);
        let original = cache.get_or_rasterize(&title).expect("first title");
        assert_eq!(cache.get_or_rasterize(&title).unwrap(), original);
        cache.get_or_rasterize(&callout).expect("callout");
        let rerendered = cache.get_or_rasterize(&title).expect("evicted title");
        assert_eq!(rerendered, original, "eviction must not alter RGBA output");
        assert_eq!(
            cache.stats(),
            TextCacheStats {
                hits: 1,
                misses: 3,
                evictions: 2,
                entries: 1,
                bytes: original.rgba.len(),
            }
        );
    }

    #[test]
    fn byte_budget_evicts_before_entry_limit() {
        let font = fixture_font();
        let mut cache = TextLayerCache::new(
            &font,
            TextCacheLimits {
                max_entries: 8,
                max_bytes: 64 * 20 * 4,
            },
        );
        let first = node("first", 64, 20);
        let second = node("second", 64, 20);
        cache.get_or_rasterize(&first).unwrap();
        let second_frame = cache.get_or_rasterize(&second).unwrap();
        assert_eq!(cache.stats().entries, 1);
        assert_eq!(cache.stats().bytes, second_frame.rgba.len());
        assert_eq!(cache.stats().evictions, 1);
        cache.get_or_rasterize(&second).unwrap();
        assert_eq!(cache.stats().hits, 1);
    }
}
