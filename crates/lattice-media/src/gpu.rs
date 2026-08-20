//! Persistent DX12 offscreen compositor.
//!
//! `GpuRendererRuntime` owns the adapter, device, queue, pipelines, and reusable
//! readback target. A `GpuCompositor` is installed into a `SampleSession`, so
//! those expensive objects survive every sample/export frame in that session.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

#[cfg(any(windows, test))]
use std::sync::{Arc, Mutex, MutexGuard};

use lattice_core::{
    BlendMode, Canvas, GroupNode, Rect, RenderNode, RenderScene, Rgba, ShapeKind, TextNode, Time,
    Transform,
};

#[cfg(windows)]
use crate::backend::RendererInitStage;
use crate::backend::{
    FrameRenderer, RawFrame, RendererInitError, RendererRenderError, RendererRequest,
    RendererSelection, VideoDecoder,
};
use crate::export::ExportError;
#[cfg(test)]
use crate::export::PreviewOptions;
use crate::font::FontResolution;
use crate::text::{TextCacheLimits, TextCacheStats, TextLayerCache};

#[derive(Clone, Debug, PartialEq, Eq)]
enum DrawSource {
    Texture(RawFrame),
    Solid(Rgba),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DrawCommand {
    source: DrawSource,
    bounds: Rect,
    transform_pivot: Rect,
    transform: Transform,
    opacity: u8,
    clip: Option<Rect>,
    blend: BlendMode,
}

#[cfg(any(windows, test))]
#[derive(Clone, Default)]
struct RuntimeFailureLatch {
    failure: Arc<Mutex<Option<RendererRenderError>>>,
}

#[cfg(any(windows, test))]
impl RuntimeFailureLatch {
    fn record(&self, failure: RendererRenderError) {
        let mut slot = self.slot();
        if slot.is_none() {
            *slot = Some(failure);
        }
    }

    fn failure(&self) -> Option<RendererRenderError> {
        self.slot().clone()
    }

    fn check(&self) -> Result<(), RendererRenderError> {
        self.failure().map_or(Ok(()), Err)
    }

    fn slot(&self) -> MutexGuard<'_, Option<RendererRenderError>> {
        self.failure
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[cfg(any(windows, test))]
fn normalized_adapter_selector(selector: &str) -> Option<String> {
    let selector = selector.trim();
    (!selector.is_empty()).then(|| selector.to_lowercase())
}

#[cfg(any(windows, test))]
fn matching_adapter_index(selector: &str, adapter_names: &[String]) -> Option<usize> {
    let selector = normalized_adapter_selector(selector)?;
    adapter_names
        .iter()
        .position(|name| name.to_lowercase().contains(&selector))
}

#[cfg(any(windows, test))]
fn adapter_selector_unavailable(selector: &str, adapter_names: &[String]) -> String {
    let mut available = adapter_names.to_vec();
    available.sort_by_key(|name| name.to_lowercase());
    available.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    let available = if available.is_empty() {
        "none".into()
    } else {
        available
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "LATTICE_DX12_ADAPTER `{}` matched no DX12 adapter; available DX12 adapters: {available}",
        selector.trim()
    )
}

#[cfg(any(windows, test))]
fn adapter_selector_init_error(selector: &str, adapter_names: &[String]) -> RendererInitError {
    RendererInitError::Unavailable {
        selection: RendererSelection::unavailable(
            RendererRequest::RequireGpuDx12,
            adapter_selector_unavailable(selector, adapter_names),
        ),
    }
}

/// Adapter/device/pipeline state shared by every frame in a GPU sample session.
pub struct GpuRendererRuntime {
    adapter_name: String,
    rendered_frames: u64,
    #[cfg(windows)]
    inner: Dx12Runtime,
}

impl GpuRendererRuntime {
    /// Creates a runtime that can only select Direct3D 12.
    ///
    /// No other wgpu backend is enabled or requested, and non-Windows targets
    /// return a typed unavailable error instead of choosing another backend.
    pub fn new_dx12() -> Result<Self, RendererInitError> {
        #[cfg(not(windows))]
        {
            Err(RendererInitError::Unavailable {
                selection: RendererSelection::unavailable(
                    RendererRequest::RequireGpuDx12,
                    "DX12 is only available on Windows",
                ),
            })
        }

        #[cfg(windows)]
        {
            let inner = pollster::block_on(Dx12Runtime::new())?;
            let adapter_name = inner.adapter_name.clone();
            Ok(Self {
                adapter_name,
                rendered_frames: 0,
                inner,
            })
        }
    }

    #[must_use]
    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    /// Number of frames submitted by this persistent runtime.
    #[must_use]
    pub fn rendered_frames(&self) -> u64 {
        self.rendered_frames
    }

    fn check_failure(&self) -> Result<(), RendererRenderError> {
        #[cfg(not(windows))]
        {
            Ok(())
        }

        #[cfg(windows)]
        {
            self.inner.failure_latch.check()
        }
    }

    #[cfg(all(test, windows))]
    fn inject_failure(&self, failure: RendererRenderError) {
        self.inner.failure_latch.record(failure);
    }

    fn render(
        &mut self,
        canvas: Canvas,
        commands: &[DrawCommand],
    ) -> Result<RawFrame, RendererRenderError> {
        #[cfg(not(windows))]
        {
            let _ = (canvas, commands);
            return Err(RendererRenderError::Validation {
                message: "DX12 runtime cannot render on a non-Windows target".into(),
            });
        }

        #[cfg(windows)]
        let frame = self.inner.render(canvas, commands)?;

        #[cfg(windows)]
        {
            self.rendered_frames = self.rendered_frames.saturating_add(1);
            Ok(frame)
        }
    }
}

/// Scene compositor backed by one persistent [`GpuRendererRuntime`].
pub struct GpuCompositor {
    runtime: GpuRendererRuntime,
    font: Option<FontResolution>,
    text_cache: Option<TextLayerCache>,
}

impl GpuCompositor {
    pub fn new_dx12(font: Option<FontResolution>) -> Result<Self, RendererInitError> {
        Self::new_dx12_with_text_cache_limits(font, TextCacheLimits::default())
    }

    pub fn new_dx12_with_text_cache_limits(
        font: Option<FontResolution>,
        limits: TextCacheLimits,
    ) -> Result<Self, RendererInitError> {
        let runtime = GpuRendererRuntime::new_dx12()?;
        let text_cache = font.as_ref().map(|font| TextLayerCache::new(font, limits));
        Ok(Self {
            runtime,
            font,
            text_cache,
        })
    }

    #[must_use]
    pub fn selection(&self) -> RendererSelection {
        RendererSelection::gpu_dx12(self.runtime.adapter_name())
    }

    #[must_use]
    pub fn rendered_frames(&self) -> u64 {
        self.runtime.rendered_frames()
    }

    #[must_use]
    pub fn text_cache_stats(&self) -> TextCacheStats {
        self.text_cache
            .as_ref()
            .map_or_else(TextCacheStats::default, TextLayerCache::stats)
    }
}

impl FrameRenderer for GpuCompositor {
    fn render(
        &mut self,
        scene: &RenderScene,
        sampler: &mut dyn VideoDecoder,
    ) -> Result<RawFrame, ExportError> {
        self.runtime.check_failure()?;
        if scene.has_text() && self.text_cache.is_none() {
            return Err(ExportError::MissingFont);
        }
        let commands =
            collect_draw_commands(scene, sampler, &mut self.text_cache, self.font.as_ref())?;
        self.runtime
            .render(scene.canvas, &commands)
            .map_err(Into::into)
    }
}

fn collect_draw_commands(
    scene: &RenderScene,
    sampler: &mut dyn VideoDecoder,
    text_cache: &mut Option<TextLayerCache>,
    font: Option<&FontResolution>,
) -> Result<Vec<DrawCommand>, ExportError> {
    let mut collector = CommandCollector {
        canvas: scene.canvas,
        sampler,
        text_cache,
        font,
        commands: Vec::new(),
    };
    let mut nodes: Vec<&RenderNode> = scene.nodes.iter().collect();
    nodes.sort_by_key(|node| node.z());
    for node in nodes {
        collector.node(node, Transform::IDENTITY, 100, None)?;
    }
    Ok(collector.commands)
}

struct CommandCollector<'a> {
    canvas: Canvas,
    sampler: &'a mut dyn VideoDecoder,
    text_cache: &'a mut Option<TextLayerCache>,
    font: Option<&'a FontResolution>,
    commands: Vec<DrawCommand>,
}

impl CommandCollector<'_> {
    fn node(
        &mut self,
        node: &RenderNode,
        parent: Transform,
        parent_opacity: u8,
        clip: Option<Rect>,
    ) -> Result<(), ExportError> {
        match node {
            RenderNode::Group(group) => self.group(group, parent, parent_opacity, clip),
            RenderNode::Video(video) => {
                let frame = self.sampler.sample(
                    &video.asset,
                    video.content_time,
                    video.bounds.width.max(1),
                    video.bounds.height.max(1),
                )?;
                self.commands.push(DrawCommand {
                    source: DrawSource::Texture(frame),
                    bounds: video.bounds,
                    transform_pivot: video.bounds,
                    transform: compose_transform(parent, video.props.transform),
                    opacity: mul_opacity(parent_opacity, video.props.opacity),
                    clip: intersect_clip(clip, video.props.clip),
                    blend: video.props.blend,
                });
                Ok(())
            }
            RenderNode::Image(image) => {
                let frame = self.sampler.sample(
                    &image.asset,
                    Time::ZERO,
                    image.bounds.width.max(1),
                    image.bounds.height.max(1),
                )?;
                self.commands.push(DrawCommand {
                    source: DrawSource::Texture(frame),
                    bounds: image.bounds,
                    transform_pivot: image.bounds,
                    transform: compose_transform(parent, image.props.transform),
                    opacity: mul_opacity(parent_opacity, image.props.opacity),
                    clip: intersect_clip(clip, image.props.clip),
                    blend: image.props.blend,
                });
                Ok(())
            }
            RenderNode::Shape(shape) => {
                if shape.kind != ShapeKind::Rectangle {
                    return Err(unsupported_scene(
                        "ellipse",
                        "ellipse coverage has not been implemented by the DX12 compositor",
                    ));
                }
                self.commands.push(DrawCommand {
                    source: DrawSource::Solid(shape.fill),
                    bounds: shape.bounds,
                    transform_pivot: shape.bounds,
                    transform: compose_transform(parent, shape.props.transform),
                    opacity: mul_opacity(parent_opacity, shape.props.opacity),
                    clip: intersect_clip(clip, shape.props.clip),
                    blend: shape.props.blend,
                });
                Ok(())
            }
            RenderNode::Text(text) => self.text(text, parent, parent_opacity, clip),
            RenderNode::Mask(_) => Err(unsupported_scene(
                "mask",
                "mask coverage has not been implemented by the DX12 compositor",
            )),
            RenderNode::Effect(effect) => Err(unsupported_scene(
                "effect",
                format!(
                    "effect `{}` has not been implemented by the DX12 compositor",
                    effect.name
                ),
            )),
        }
    }

    fn group(
        &mut self,
        group: &GroupNode,
        parent: Transform,
        parent_opacity: u8,
        clip: Option<Rect>,
    ) -> Result<(), ExportError> {
        let transform = compose_transform(parent, group.props.transform);
        let opacity = mul_opacity(parent_opacity, group.props.opacity);
        let clip = intersect_clip(clip, group.props.clip);
        let mut children: Vec<&RenderNode> = group.children.iter().collect();
        children.sort_by_key(|node| node.z());
        for child in children {
            self.node(child, transform, opacity, clip)?;
        }
        Ok(())
    }

    fn text(
        &mut self,
        text: &TextNode,
        parent: Transform,
        parent_opacity: u8,
        clip: Option<Rect>,
    ) -> Result<(), ExportError> {
        let opacity = mul_opacity(parent_opacity, text.props.opacity);
        if opacity == 0 || text.text.is_empty() {
            return Ok(());
        }
        if text.bounds.width == 0 || text.bounds.height == 0 {
            return Ok(());
        }
        let Some(text_cache) = self.text_cache.as_mut() else {
            return Err(ExportError::MissingFont);
        };
        if let (Some(expected), Some(loaded)) = (&text.resolved_font, self.font)
            && expected.identity != loaded.identity.identity
        {
            return Err(ExportError::StaleFont(expected.path.clone()));
        }
        let layer = text_cache.get_or_rasterize(text)?;
        self.commands.push(DrawCommand {
            source: DrawSource::Texture(layer),
            bounds: text.bounds,
            // Core currently defines title/callout scaling around the canvas
            // pivot. Keep that placement contract while uploading only the
            // node-local transparent raster.
            transform_pivot: Rect::from_canvas(self.canvas),
            transform: compose_transform(parent, text.props.transform),
            opacity,
            clip: intersect_clip(clip, text.props.clip),
            blend: text.props.blend,
        });
        Ok(())
    }
}

fn unsupported_scene(node: impl Into<String>, reason: impl Into<String>) -> ExportError {
    RendererRenderError::UnsupportedScene {
        node: node.into(),
        reason: reason.into(),
    }
    .into()
}

fn mul_opacity(a: u8, b: u8) -> u8 {
    ((u16::from(a) * u16::from(b)) / 100) as u8
}

fn compose_transform(parent: Transform, child: Transform) -> Transform {
    Transform {
        translate_x: parent.translate_x + child.translate_x,
        translate_y: parent.translate_y + child.translate_y,
        scale_x: (i64::from(parent.scale_x) * i64::from(child.scale_x) / 1000) as i32,
        scale_y: (i64::from(parent.scale_y) * i64::from(child.scale_y) / 1000) as i32,
        rotation_mdeg: parent.rotation_mdeg + child.rotation_mdeg,
    }
}

fn intersect_clip(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (None, other) | (other, None) => other,
        (Some(a), Some(b)) => {
            let x = a.x.max(b.x);
            let y = a.y.max(b.y);
            let ax2 =
                a.x.saturating_add(i32::try_from(a.width).unwrap_or(i32::MAX));
            let ay2 =
                a.y.saturating_add(i32::try_from(a.height).unwrap_or(i32::MAX));
            let bx2 =
                b.x.saturating_add(i32::try_from(b.width).unwrap_or(i32::MAX));
            let by2 =
                b.y.saturating_add(i32::try_from(b.height).unwrap_or(i32::MAX));
            let x2 = ax2.min(bx2);
            let y2 = ay2.min(by2);
            Some(Rect {
                x,
                y,
                width: u32::try_from((x2 - x).max(0)).unwrap_or(0),
                height: u32::try_from((y2 - y).max(0)).unwrap_or(0),
            })
        }
    }
}

#[cfg(windows)]
struct Dx12Runtime {
    _instance: wgpu::Instance,
    adapter_name: String,
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture_layout: wgpu::BindGroupLayout,
    texture_sampler: wgpu::Sampler,
    src_over_pipeline: wgpu::RenderPipeline,
    multiply_pipeline: wgpu::RenderPipeline,
    _white_texture: wgpu::Texture,
    white_bind_group: wgpu::BindGroup,
    targets: Option<FrameTargets>,
    failure_latch: RuntimeFailureLatch,
}

#[cfg(windows)]
struct FrameTargets {
    width: u32,
    height: u32,
    bytes_per_row: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    readback: wgpu::Buffer,
}

#[cfg(windows)]
struct GpuDraw {
    _texture: Option<wgpu::Texture>,
    bind_group: wgpu::BindGroup,
    vertex_buffer: wgpu::Buffer,
    blend: BlendMode,
    scissor: (u32, u32, u32, u32),
}

#[cfg(windows)]
fn configured_adapter_selector() -> Option<String> {
    std::env::var_os("LATTICE_DX12_ADAPTER").and_then(|selector| {
        let selector = selector.to_string_lossy();
        let selector = selector.trim();
        (!selector.is_empty()).then(|| selector.to_owned())
    })
}

#[cfg(windows)]
async fn select_dx12_adapter(
    instance: &wgpu::Instance,
) -> Result<wgpu::Adapter, RendererInitError> {
    let Some(selector) = configured_adapter_selector() else {
        return instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .map_err(|error| RendererInitError::Unavailable {
                selection: RendererSelection::unavailable(
                    RendererRequest::RequireGpuDx12,
                    format!("no high-performance DX12 adapter: {error}"),
                ),
            });
    };

    let mut adapters: Vec<_> = instance
        .enumerate_adapters(wgpu::Backends::DX12)
        .into_iter()
        .filter(|adapter| adapter.get_info().backend == wgpu::Backend::Dx12)
        .collect();
    let names: Vec<_> = adapters
        .iter()
        .map(|adapter| adapter.get_info().name)
        .collect();
    let Some(index) = matching_adapter_index(&selector, &names) else {
        return Err(adapter_selector_init_error(&selector, &names));
    };
    Ok(adapters.swap_remove(index))
}

#[cfg(windows)]
fn device_lost_error(reason: wgpu::DeviceLostReason, message: String) -> RendererRenderError {
    let reason = match reason {
        wgpu::DeviceLostReason::Unknown => "unknown",
        wgpu::DeviceLostReason::Destroyed => "destroyed",
    };
    RendererRenderError::DeviceLost {
        reason: reason.into(),
        message,
    }
}

#[cfg(windows)]
fn uncaptured_error(error: wgpu::Error) -> RendererRenderError {
    match error {
        wgpu::Error::OutOfMemory { source } => RendererRenderError::OutOfMemory {
            message: source.to_string(),
        },
        wgpu::Error::Internal { description, .. } => RendererRenderError::Internal {
            message: description,
        },
        wgpu::Error::Validation { description, .. } => RendererRenderError::Validation {
            message: description,
        },
    }
}

#[cfg(windows)]
impl Dx12Runtime {
    async fn new() -> Result<Self, RendererInitError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = select_dx12_adapter(&instance).await?;
        let info = adapter.get_info();
        if info.backend != wgpu::Backend::Dx12 {
            return Err(RendererInitError::Initialization {
                selection: RendererSelection::unavailable(
                    RendererRequest::RequireGpuDx12,
                    "wgpu selected a non-DX12 adapter",
                ),
                stage: RendererInitStage::Adapter,
                message: format!("selected backend was {}", info.backend),
            });
        }
        let adapter_name = info.name.clone();
        let selection = || {
            RendererSelection::unavailable(
                RendererRequest::RequireGpuDx12,
                format!("DX12 adapter `{adapter_name}` did not initialize"),
            )
        };
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("lattice-dx12"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| RendererInitError::Initialization {
                selection: selection(),
                stage: RendererInitStage::Device,
                message: error.to_string(),
            })?;
        let failure_latch = RuntimeFailureLatch::default();
        let device_lost_latch = failure_latch.clone();
        device.set_device_lost_callback(move |reason, message| {
            device_lost_latch.record(device_lost_error(reason, message));
        });
        let uncaptured_latch = failure_latch.clone();
        device.on_uncaptured_error(Box::new(move |error| {
            uncaptured_latch.record(uncaptured_error(error));
        }));

        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lattice-texture-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lattice-nearest-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lattice-textured-quad"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lattice-dx12-pipeline-layout"),
            bind_group_layouts: &[&texture_layout],
            push_constant_ranges: &[],
        });
        let src_over_pipeline = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            "lattice-src-over-pipeline",
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let multiply_pipeline = create_pipeline(
            &device,
            &pipeline_layout,
            &shader,
            "lattice-multiply-pipeline",
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Dst,
                    dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent::OVER,
            },
        );
        let white_texture = create_sampled_texture(&device, 1, 1, "lattice-white-texture");
        queue.write_texture(
            white_texture.as_image_copy(),
            &[255, 255, 255, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let white_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let white_bind_group = create_texture_bind_group(
            &device,
            &texture_layout,
            &white_view,
            &texture_sampler,
            "lattice-white-bind-group",
        );
        if let Some(error) = device.pop_error_scope().await {
            return Err(RendererInitError::Initialization {
                selection: selection(),
                stage: RendererInitStage::Pipeline,
                message: error.to_string(),
            });
        }
        Ok(Self {
            _instance: instance,
            adapter_name,
            device,
            queue,
            texture_layout,
            texture_sampler,
            src_over_pipeline,
            multiply_pipeline,
            _white_texture: white_texture,
            white_bind_group,
            targets: None,
            failure_latch,
        })
    }

    fn render(
        &mut self,
        canvas: Canvas,
        commands: &[DrawCommand],
    ) -> Result<RawFrame, RendererRenderError> {
        self.failure_latch.check()?;
        let width = canvas.width.max(1);
        let height = canvas.height.max(1);
        self.ensure_targets(width, height);
        for command in commands {
            if let DrawSource::Texture(frame) = &command.source {
                validate_frame(frame)?;
            }
        }
        self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut draws = Vec::with_capacity(commands.len());
        for command in commands {
            if command.opacity == 0 || command.bounds.width == 0 || command.bounds.height == 0 {
                continue;
            }
            let Some(scissor) = command_scissor(canvas, command.clip) else {
                continue;
            };
            let vertices = command_vertices(canvas, command);
            let vertex_bytes = f32_bytes(&vertices);
            let vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("lattice-draw-vertices"),
                size: vertex_bytes.len() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.queue.write_buffer(&vertex_buffer, 0, &vertex_bytes);
            let (texture, bind_group) = match &command.source {
                DrawSource::Solid(_) => (None, self.white_bind_group.clone()),
                DrawSource::Texture(frame) => {
                    let texture = create_sampled_texture(
                        &self.device,
                        frame.width,
                        frame.height,
                        "lattice-frame-texture",
                    );
                    self.queue.write_texture(
                        texture.as_image_copy(),
                        &frame.rgba,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(frame.width * 4),
                            rows_per_image: Some(frame.height),
                        },
                        wgpu::Extent3d {
                            width: frame.width,
                            height: frame.height,
                            depth_or_array_layers: 1,
                        },
                    );
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let bind_group = create_texture_bind_group(
                        &self.device,
                        &self.texture_layout,
                        &view,
                        &self.texture_sampler,
                        "lattice-frame-bind-group",
                    );
                    (Some(texture), bind_group)
                }
            };
            draws.push(GpuDraw {
                _texture: texture,
                bind_group,
                vertex_buffer,
                blend: command.blend,
                scissor,
            });
        }
        self.failure_latch.check()?;

        let targets = self.targets.as_ref().expect("targets initialized");
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("lattice-dx12-frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("lattice-dx12-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &targets.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            for draw in &draws {
                let pipeline = match draw.blend {
                    BlendMode::SrcOver => &self.src_over_pipeline,
                    BlendMode::Multiply => &self.multiply_pipeline,
                };
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &draw.bind_group, &[]);
                pass.set_scissor_rect(
                    draw.scissor.0,
                    draw.scissor.1,
                    draw.scissor.2,
                    draw.scissor.3,
                );
                pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
                pass.draw(0..6, 0..1);
            }
        }
        encoder.copy_texture_to_buffer(
            targets.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &targets.readback,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(targets.bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = targets.readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        if let Err(error) = self.device.poll(wgpu::PollType::Wait) {
            if let Some(failure) = self.failure_latch.failure() {
                return Err(failure);
            }
            return Err(RendererRenderError::DevicePoll {
                message: error.to_string(),
            });
        }
        self.failure_latch.check()?;
        let map_result = receiver
            .recv()
            .map_err(|error| RendererRenderError::Readback {
                message: error.to_string(),
            })?;
        self.failure_latch.check()?;
        map_result.map_err(|error| RendererRenderError::Readback {
            message: error.to_string(),
        })?;
        let data = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            let start = (y * targets.bytes_per_row) as usize;
            let end = start + (width * 4) as usize;
            rgba.extend_from_slice(&data[start..end]);
        }
        drop(data);
        targets.readback.unmap();
        let validation_error = pollster::block_on(self.device.pop_error_scope());
        self.failure_latch.check()?;
        if let Some(error) = validation_error {
            return Err(RendererRenderError::Validation {
                message: error.to_string(),
            });
        }
        Ok(RawFrame {
            width: canvas.width,
            height: canvas.height,
            rgba,
        })
    }

    fn ensure_targets(&mut self, width: u32, height: u32) {
        let matches = self
            .targets
            .as_ref()
            .is_some_and(|targets| targets.width == width && targets.height == height);
        if matches {
            return;
        }
        let bytes_per_row = (width * 4).div_ceil(256) * 256;
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("lattice-dx12-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lattice-dx12-readback"),
            size: u64::from(bytes_per_row) * u64::from(height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        self.targets = Some(FrameTargets {
            width,
            height,
            bytes_per_row,
            texture,
            view,
            readback,
        });
    }
}

#[cfg(windows)]
fn create_sampled_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &str,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

#[cfg(windows)]
fn create_texture_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}

#[cfg(windows)]
fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    label: &str,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 32,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 8,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 2,
                    },
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

#[cfg(windows)]
fn validate_frame(frame: &RawFrame) -> Result<(), RendererRenderError> {
    let expected_bytes = (frame.width as usize)
        .saturating_mul(frame.height as usize)
        .saturating_mul(4);
    if frame.width == 0 || frame.height == 0 || frame.rgba.len() != expected_bytes {
        return Err(RendererRenderError::InvalidFrame {
            width: frame.width,
            height: frame.height,
            expected_bytes,
            actual_bytes: frame.rgba.len(),
        });
    }
    Ok(())
}

#[cfg(windows)]
fn command_scissor(canvas: Canvas, clip: Option<Rect>) -> Option<(u32, u32, u32, u32)> {
    let (mut x0, mut y0, mut x1, mut y1) = (
        0i64,
        0i64,
        i64::from(canvas.width),
        i64::from(canvas.height),
    );
    if let Some(clip) = clip {
        x0 = x0.max(i64::from(clip.x));
        y0 = y0.max(i64::from(clip.y));
        x1 = x1.min(i64::from(clip.x) + i64::from(clip.width));
        y1 = y1.min(i64::from(clip.y) + i64::from(clip.height));
    }
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((
        u32::try_from(x0).ok()?,
        u32::try_from(y0).ok()?,
        u32::try_from(x1 - x0).ok()?,
        u32::try_from(y1 - y0).ok()?,
    ))
}

#[cfg(windows)]
fn command_vertices(canvas: Canvas, command: &DrawCommand) -> Vec<f32> {
    let x0 = f64::from(command.bounds.x);
    let y0 = f64::from(command.bounds.y);
    let x1 = x0 + f64::from(command.bounds.width);
    let y1 = y0 + f64::from(command.bounds.height);
    let corners = [
        (x0, y0, 0.0, 0.0),
        (x1, y0, 1.0, 0.0),
        (x0, y1, 0.0, 1.0),
        (x1, y0, 1.0, 0.0),
        (x1, y1, 1.0, 1.0),
        (x0, y1, 0.0, 1.0),
    ];
    let color = match command.source {
        DrawSource::Texture(_) => [1.0, 1.0, 1.0, f32::from(command.opacity) / 100.0],
        DrawSource::Solid(fill) => [
            f32::from(fill.r) / 255.0,
            f32::from(fill.g) / 255.0,
            f32::from(fill.b) / 255.0,
            f32::from(fill.a) / 255.0 * f32::from(command.opacity) / 100.0,
        ],
    };
    let mut vertices = Vec::with_capacity(48);
    for (x, y, u, v) in corners {
        let (x, y) = transformed_point(command.transform_pivot, command.transform, x, y);
        // CpuCompositor evaluates each destination pixel at its integer lattice
        // coordinate. D3D rasterization evaluates fragments at half-pixel centers,
        // so move geometry by half a pixel to make both backends invert the same
        // transform/sample coordinate.
        let x = x + 0.5;
        let y = y + 0.5;
        vertices.extend_from_slice(&[
            (x / f64::from(canvas.width.max(1)) * 2.0 - 1.0) as f32,
            (1.0 - y / f64::from(canvas.height.max(1)) * 2.0) as f32,
            u,
            v,
            color[0],
            color[1],
            color[2],
            color[3],
        ]);
    }
    vertices
}

#[cfg(windows)]
fn transformed_point(bounds: Rect, transform: Transform, x: f64, y: f64) -> (f64, f64) {
    let cx = f64::from(bounds.x) + f64::from(bounds.width) / 2.0;
    let cy = f64::from(bounds.y) + f64::from(bounds.height) / 2.0;
    let sx = (f64::from(transform.scale_x) / 1000.0).max(0.001);
    let sy = (f64::from(transform.scale_y) / 1000.0).max(0.001);
    let theta = f64::from(transform.rotation_mdeg) * std::f64::consts::PI / 180_000.0;
    let (sin, cos) = theta.sin_cos();
    let dx = (x - cx) * sx;
    let dy = (y - cy) * sy;
    (
        cos * dx - sin * dy + cx + f64::from(transform.translate_x),
        sin * dx + cos * dy + cy + f64::from(transform.translate_y),
    )
}

#[cfg(windows)]
fn f32_bytes(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

#[cfg(windows)]
const SHADER: &str = r"
struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};
struct VertexOutput {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};
@group(0) @binding(0) var frame_texture: texture_2d<f32>;
@group(0) @binding(1) var frame_sampler: sampler;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.pos = vec4<f32>(input.pos, 0.0, 1.0);
    out.uv = input.uv;
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let size = textureDimensions(frame_texture);
    let max_coord = vec2<i32>(size) - vec2<i32>(1, 1);
    let source = input.uv * vec2<f32>(size);
    let rounded = round(source);
    let snapped = select(source, rounded, abs(source - rounded) <= vec2<f32>(0.0001, 0.0001));
    let texel = clamp(
        vec2<i32>(floor(snapped)),
        vec2<i32>(0, 0),
        max_coord,
    );
    let sampled = textureLoad(frame_texture, texel, 0) * input.color;
    // CPU compositing quantizes effective alpha with integer division before
    // blending. Mirror that step so opacity is within one RGBA8 value.
    let alpha = floor(sampled.a * 255.0 + 0.0001) / 255.0;
    return vec4<f32>(sampled.rgb, alpha);
}
";

/// Compatibility one-shot probe. Preview/export uses `GpuCompositor` retained by
/// `SampleSession`; callers that repeatedly render should do the same.
pub fn render_offscreen(scene: &RenderScene, video: &RawFrame) -> Result<RawFrame, String> {
    struct SuppliedFrame(RawFrame);

    impl VideoDecoder for SuppliedFrame {
        fn sample(
            &mut self,
            _asset: &lattice_core::AssetRef,
            _content_time: Time,
            _width: u32,
            _height: u32,
        ) -> Result<RawFrame, ExportError> {
            Ok(self.0.clone())
        }
    }

    let mut compositor = GpuCompositor::new_dx12(None).map_err(|error| error.to_string())?;
    let mut sampler = SuppliedFrame(video.clone());
    compositor
        .render(scene, &mut sampler)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use lattice_core::{
        AssetRef, EffectNode, FontSpec, ImageNode, MaskNode, MediaLocator, NodeProps, ShapeKind,
        ShapeNode, TimeMap, VideoNode,
    };

    struct GradientDecoder {
        frame: RawFrame,
        calls: usize,
    }

    impl VideoDecoder for GradientDecoder {
        fn sample(
            &mut self,
            _asset: &AssetRef,
            _content_time: Time,
            _width: u32,
            _height: u32,
        ) -> Result<RawFrame, ExportError> {
            self.calls += 1;
            Ok(self.frame.clone())
        }
    }

    #[derive(Default)]
    struct RequestedPatternDecoder {
        calls: usize,
    }

    impl VideoDecoder for RequestedPatternDecoder {
        fn sample(
            &mut self,
            asset: &AssetRef,
            _content_time: Time,
            width: u32,
            height: u32,
        ) -> Result<RawFrame, ExportError> {
            self.calls += 1;
            let seed = if asset.media_name == "image" { 79 } else { 0 };
            Ok(pattern(width, height, seed))
        }
    }

    fn video_node(canvas: Canvas) -> RenderNode {
        RenderNode::Video(VideoNode {
            props: NodeProps::opaque(0),
            bounds: Rect::from_canvas(canvas),
            asset: AssetRef {
                media_name: "video".into(),
                locator: MediaLocator::File {
                    path: "video.mp4".into(),
                },
            },
            content_time: Time::ZERO,
            hold: false,
            time_map: TimeMap::identity(Time::ZERO, Time::ONE),
        })
    }

    fn gradient(width: u32, height: u32) -> RawFrame {
        let mut frame = RawFrame::filled(width, height, 0, 0, 0, 255);
        for y in 0..height {
            for x in 0..width {
                frame.set_pixel(
                    x,
                    y,
                    [
                        u8::try_from(x * 17).unwrap_or(255),
                        u8::try_from(y * 31).unwrap_or(255),
                        u8::try_from((x + y) * 13).unwrap_or(255),
                        255,
                    ],
                );
            }
        }
        frame
    }

    fn pattern(width: u32, height: u32, seed: u32) -> RawFrame {
        let mut frame = RawFrame::filled(width, height, 0, 0, 0, 255);
        for y in 0..height {
            for x in 0..width {
                frame.set_pixel(
                    x,
                    y,
                    [
                        ((x * 17 + y * 3 + seed) % 251) as u8,
                        ((x * 5 + y * 29 + seed * 2) % 251) as u8,
                        ((x * 11 + y * 7 + seed * 3) % 251) as u8,
                        255,
                    ],
                );
            }
        }
        frame
    }

    fn shape_node(kind: ShapeKind) -> RenderNode {
        RenderNode::Shape(ShapeNode {
            props: NodeProps::opaque(1),
            bounds: Rect {
                x: 1,
                y: 1,
                width: 4,
                height: 3,
            },
            kind,
            fill: Rgba::YELLOW,
        })
    }

    fn fixture_font() -> FontResolution {
        let path = crate::font::fixture_font_path().expect("repository fixture font");
        crate::font::resolve_font(
            &FontSpec::preview_sans(18),
            Path::new("."),
            None,
            Some(&path),
        )
        .expect("resolve fixture font")
    }

    fn text_node(font: &FontResolution, text: &str, bounds: Rect, props: NodeProps) -> RenderNode {
        RenderNode::Text(TextNode {
            props,
            bounds,
            text: text.into(),
            font: FontSpec::preview_sans(18),
            resolved_font: Some(font.identity.clone()),
            color: Rgba::WHITE,
        })
    }

    fn assert_unsupported(node: RenderNode, expected: &str) {
        let scene = RenderScene {
            canvas: Canvas {
                width: 8,
                height: 6,
            },
            nodes: vec![node],
        };
        let mut decoder = GradientDecoder {
            frame: gradient(8, 6),
            calls: 0,
        };
        let error = collect_draw_commands(&scene, &mut decoder, &mut None, None)
            .expect_err("unsupported scene must fail before rendering");
        match error {
            ExportError::RendererRender(RendererRenderError::UnsupportedScene { node, reason }) => {
                assert_eq!(node, expected);
                assert!(reason.contains(expected), "{reason}");
            }
            other => panic!("unexpected error for {expected}: {other}"),
        }
        assert_eq!(decoder.calls, 0, "unsupported scene must not decode media");
    }

    #[test]
    fn explicit_adapter_selector_matches_case_insensitive_substrings() {
        let names = vec![
            "NVIDIA GeForce RTX 4090".to_owned(),
            "AMD Radeon 780M Graphics".to_owned(),
        ];
        assert_eq!(matching_adapter_index("nViDiA", &names), Some(0));
        assert_eq!(matching_adapter_index("  radeon 780m ", &names), Some(1));
        assert_eq!(matching_adapter_index("", &names), None);
        assert_eq!(matching_adapter_index("intel", &names), None);
        let unavailable = adapter_selector_unavailable(" Intel ", &names);
        assert!(unavailable.contains("LATTICE_DX12_ADAPTER `Intel`"));
        assert!(unavailable.contains("`AMD Radeon 780M Graphics`"));
        assert!(unavailable.contains("`NVIDIA GeForce RTX 4090`"));
        let error = adapter_selector_init_error(" Intel ", &names);
        let RendererInitError::Unavailable { selection } = error else {
            panic!("selector mismatch must be typed unavailable")
        };
        assert_eq!(selection.requested, RendererRequest::RequireGpuDx12);
        assert_eq!(selection.active, None);
        assert_eq!(selection.adapter, None);
        assert!(selection.reason.contains("available DX12 adapters"));
    }

    #[test]
    fn runtime_failure_latch_classifies_and_persists_first_failure() {
        let failures = [
            RendererRenderError::DeviceLost {
                reason: "unknown".into(),
                message: "test device lost".into(),
            },
            RendererRenderError::OutOfMemory {
                message: "test OOM".into(),
            },
            RendererRenderError::Internal {
                message: "test internal".into(),
            },
        ];
        assert_eq!(
            failures
                .iter()
                .map(RendererRenderError::kind)
                .collect::<Vec<_>>(),
            ["device_lost", "out_of_memory", "internal"]
        );
        for failure in failures {
            let latch = RuntimeFailureLatch::default();
            latch.record(failure.clone());
            assert_eq!(latch.check(), Err(failure.clone()));
            assert_eq!(latch.check(), Err(failure.clone()));
            assert!(failure.is_persistent());
            latch.record(RendererRenderError::Internal {
                message: "later failure must not replace the first".into(),
            });
            assert_eq!(latch.failure(), Some(failure));
        }
    }

    #[cfg(windows)]
    #[test]
    fn wgpu_callbacks_map_to_typed_runtime_failures() {
        let source =
            || -> wgpu::ErrorSource { Box::new(std::io::Error::other("injected wgpu failure")) };
        assert!(matches!(
            uncaptured_error(wgpu::Error::OutOfMemory { source: source() }),
            RendererRenderError::OutOfMemory { .. }
        ));
        assert_eq!(
            uncaptured_error(wgpu::Error::Internal {
                source: source(),
                description: "injected internal".into(),
            }),
            RendererRenderError::Internal {
                message: "injected internal".into(),
            }
        );
        assert_eq!(
            device_lost_error(wgpu::DeviceLostReason::Unknown, "driver reset".into()),
            RendererRenderError::DeviceLost {
                reason: "unknown".into(),
                message: "driver reset".into(),
            }
        );
    }

    #[test]
    fn unsupported_scene_nodes_fail_typed_instead_of_silently_drawing() {
        assert_unsupported(shape_node(ShapeKind::Ellipse), "ellipse");
        assert_unsupported(
            RenderNode::Mask(MaskNode {
                props: NodeProps::opaque(1),
                mask: Box::new(shape_node(ShapeKind::Rectangle)),
                content: Box::new(shape_node(ShapeKind::Rectangle)),
            }),
            "mask",
        );
        assert_unsupported(
            RenderNode::Effect(EffectNode {
                props: NodeProps::opaque(1),
                name: "blur".into(),
                child: Box::new(shape_node(ShapeKind::Rectangle)),
            }),
            "effect",
        );
    }

    #[test]
    fn text_commands_reuse_node_local_raster_across_compositing_changes() {
        let canvas = Canvas {
            width: 320,
            height: 180,
        };
        let font = fixture_font();
        let bounds = Rect {
            x: 12,
            y: 34,
            width: 120,
            height: 32,
        };
        let scene = RenderScene {
            canvas,
            nodes: vec![text_node(
                &font,
                "Stable title",
                bounds,
                NodeProps {
                    transform: Transform::translate(3, 4),
                    opacity: 70,
                    clip: Some(Rect {
                        x: 20,
                        y: 35,
                        width: 80,
                        height: 20,
                    }),
                    z: 2,
                    blend: BlendMode::SrcOver,
                },
            )],
        };
        let mut text_cache = Some(TextLayerCache::new(&font, TextCacheLimits::default()));
        let mut decoder = RequestedPatternDecoder::default();
        let first =
            collect_draw_commands(&scene, &mut decoder, &mut text_cache, Some(&font)).unwrap();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].bounds, bounds);
        assert_eq!(first[0].transform_pivot, Rect::from_canvas(canvas));
        let DrawSource::Texture(layer) = &first[0].source else {
            panic!("text must upload a local texture");
        };
        assert_eq!((layer.width, layer.height), (bounds.width, bounds.height));
        assert!(
            layer.rgba.len() < (canvas.width * canvas.height * 4) as usize,
            "text layer must not allocate a full canvas"
        );
        assert!(
            layer.rgba.chunks_exact(4).any(|pixel| pixel[3] != 0),
            "local text layer must contain glyph coverage"
        );

        let mut moved = scene.clone();
        let RenderNode::Text(text) = &mut moved.nodes[0] else {
            unreachable!()
        };
        text.bounds.x = 80;
        text.bounds.y = 90;
        text.props.transform = Transform {
            rotation_mdeg: 90_000,
            ..Transform::IDENTITY
        };
        text.props.opacity = 35;
        text.props.clip = None;
        collect_draw_commands(&moved, &mut decoder, &mut text_cache, Some(&font)).unwrap();
        assert_eq!(
            text_cache.as_ref().unwrap().stats(),
            TextCacheStats {
                hits: 1,
                misses: 1,
                evictions: 0,
                entries: 1,
                bytes: (bounds.width * bounds.height * 4) as usize,
            }
        );
        assert_eq!(decoder.calls, 0);
    }

    #[test]
    fn command_integration_keeps_full_video_texture_and_every_shape() {
        let canvas = Canvas {
            width: 8,
            height: 4,
        };
        let input = gradient(canvas.width, canvas.height);
        let scene = RenderScene {
            canvas,
            nodes: vec![
                video_node(canvas),
                RenderNode::Shape(ShapeNode {
                    props: NodeProps::opaque(1),
                    bounds: Rect {
                        x: 0,
                        y: 0,
                        width: 2,
                        height: 2,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::YELLOW,
                }),
                RenderNode::Shape(ShapeNode {
                    props: NodeProps::opaque(2),
                    bounds: Rect {
                        x: 6,
                        y: 2,
                        width: 2,
                        height: 2,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::CYAN,
                }),
            ],
        };
        let mut decoder = GradientDecoder {
            frame: input.clone(),
            calls: 0,
        };
        let commands = collect_draw_commands(&scene, &mut decoder, &mut None, None).unwrap();
        assert_eq!(decoder.calls, 1);
        assert_eq!(commands.len(), 3, "video plus both shapes");
        let DrawSource::Texture(uploaded) = &commands[0].source else {
            panic!("first command must be the video texture");
        };
        assert_eq!(uploaded, &input, "all decoded pixels must reach upload");
        assert_ne!(uploaded.pixel(0, 0), uploaded.pixel(7, 3));
        assert!(matches!(
            commands[1].source,
            DrawSource::Solid(Rgba::YELLOW)
        ));
        assert!(matches!(commands[2].source, DrawSource::Solid(Rgba::CYAN)));
    }

    #[test]
    fn nested_group_semantics_are_flattened_deterministically() {
        let canvas = Canvas {
            width: 32,
            height: 18,
        };
        let child = RenderNode::Shape(ShapeNode {
            props: NodeProps {
                transform: Transform::translate(3, 4),
                opacity: 50,
                clip: Some(Rect {
                    x: 4,
                    y: 4,
                    width: 8,
                    height: 8,
                }),
                z: 1,
                blend: BlendMode::Multiply,
            },
            bounds: Rect {
                x: 0,
                y: 0,
                width: 12,
                height: 12,
            },
            kind: ShapeKind::Rectangle,
            fill: Rgba::CYAN,
        });
        let scene = RenderScene {
            canvas,
            nodes: vec![RenderNode::Group(GroupNode {
                props: NodeProps {
                    transform: Transform::translate(5, 6),
                    opacity: 80,
                    clip: Some(Rect {
                        x: 2,
                        y: 2,
                        width: 8,
                        height: 8,
                    }),
                    z: 0,
                    blend: BlendMode::SrcOver,
                },
                children: vec![child],
            })],
        };
        let mut decoder = GradientDecoder {
            frame: gradient(1, 1),
            calls: 0,
        };
        let commands = collect_draw_commands(&scene, &mut decoder, &mut None, None).unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].transform.translate_x, 8);
        assert_eq!(commands[0].transform.translate_y, 10);
        assert_eq!(commands[0].opacity, 40);
        assert_eq!(
            commands[0].clip,
            Some(Rect {
                x: 4,
                y: 4,
                width: 6,
                height: 6,
            })
        );
        assert_eq!(commands[0].blend, BlendMode::Multiply);
    }

    #[cfg(windows)]
    fn dx12_hardware_test_compositor() -> Option<GpuCompositor> {
        dx12_hardware_test_compositor_with_font(None)
    }

    #[cfg(windows)]
    fn dx12_hardware_test_compositor_with_font(
        font: Option<FontResolution>,
    ) -> Option<GpuCompositor> {
        match GpuCompositor::new_dx12(font) {
            Ok(gpu) => Some(gpu),
            Err(error) => {
                let explicitly_required = std::env::var_os("LATTICE_REQUIRE_DX12_TESTS").is_some();
                if matches!(&error, RendererInitError::Unavailable { .. }) && !explicitly_required {
                    eprintln!(
                        "DX12 conformance soft-skip: {error}; set \
                         LATTICE_REQUIRE_DX12_TESTS=1 to make missing hardware fail"
                    );
                    return None;
                }
                panic!("DX12 conformance gate failed: {error}");
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn dx12_session_selection_reports_the_selected_adapter() {
        let Some(gpu) = dx12_hardware_test_compositor() else {
            return;
        };
        let selection = gpu.selection();
        assert_eq!(
            selection.adapter.as_deref(),
            Some(gpu.runtime.adapter_name())
        );
        if let Some(selector) = configured_adapter_selector() {
            assert_eq!(
                matching_adapter_index(&selector, &[gpu.runtime.adapter_name().to_owned()]),
                Some(0),
                "selected adapter must honor LATTICE_DX12_ADAPTER={selector}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn injected_persistent_failure_preempts_sampling_and_stays_latched() {
        let Some(mut gpu) = dx12_hardware_test_compositor() else {
            return;
        };
        let expected = RendererRenderError::OutOfMemory {
            message: "injected persistent OOM".into(),
        };
        gpu.runtime.inject_failure(expected.clone());
        let canvas = Canvas {
            width: 16,
            height: 9,
        };
        let scene = RenderScene {
            canvas,
            nodes: vec![video_node(canvas)],
        };
        let mut decoder = RequestedPatternDecoder::default();
        for _ in 0..2 {
            let error = gpu
                .render(&scene, &mut decoder)
                .expect_err("latched runtime must reject every render");
            match error {
                ExportError::RendererRender(actual) => assert_eq!(actual, expected),
                other => panic!("unexpected latched error: {other}"),
            }
        }
        assert_eq!(decoder.calls, 0, "failure must preempt scene sampling");
        assert_eq!(gpu.rendered_frames(), 0);
    }

    #[cfg(windows)]
    fn assert_frames_within_one(gpu: &RawFrame, cpu: &RawFrame) {
        assert_eq!((gpu.width, gpu.height), (cpu.width, cpu.height));
        assert_eq!(gpu.rgba.len(), cpu.rgba.len());
        let (index, difference) = gpu
            .rgba
            .iter()
            .zip(&cpu.rgba)
            .enumerate()
            .map(|(index, (gpu, cpu))| (index, gpu.abs_diff(*cpu)))
            .max_by_key(|(_, difference)| *difference)
            .unwrap_or((0, 0));
        let pixel = index / 4;
        let x = pixel % gpu.width as usize;
        let y = pixel / gpu.width as usize;
        let channel = ["r", "g", "b", "a"][index % 4];
        assert!(
            difference <= 1,
            "GPU/CPU difference {difference} at ({x},{y}) channel {channel}: gpu={} cpu={}",
            gpu.rgba[index],
            cpu.rgba[index]
        );
    }

    #[cfg(windows)]
    fn frame_fingerprint(frame: &RawFrame) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for byte in frame
            .width
            .to_le_bytes()
            .iter()
            .chain(frame.height.to_le_bytes().iter())
            .chain(&frame.rgba)
        {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
        hash
    }

    #[cfg(windows)]
    #[test]
    fn dx12_title_and_callout_match_fixture_font_cpu_golden() {
        let font = fixture_font();
        let canvas = Canvas {
            width: 160,
            height: 90,
        };
        let title_bounds = Rect {
            x: 4,
            y: 58,
            width: 112,
            height: 26,
        };
        let callout_bounds = Rect {
            x: 24,
            y: 10,
            width: 104,
            height: 24,
        };
        let scene = RenderScene {
            canvas,
            nodes: vec![
                RenderNode::Shape(ShapeNode {
                    props: NodeProps::opaque(0),
                    bounds: Rect {
                        x: 0,
                        y: 84,
                        width: 120,
                        height: 6,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::YELLOW,
                }),
                text_node(
                    &font,
                    "Lattice Title",
                    title_bounds,
                    NodeProps {
                        opacity: 82,
                        clip: Some(Rect {
                            x: 4,
                            y: 58,
                            width: 108,
                            height: 24,
                        }),
                        ..NodeProps::opaque(1)
                    },
                ),
                RenderNode::Shape(ShapeNode {
                    props: NodeProps::opaque(2),
                    bounds: Rect {
                        x: 20,
                        y: 4,
                        width: 108,
                        height: 6,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::CYAN,
                }),
                text_node(
                    &font,
                    "注目 Callout",
                    callout_bounds,
                    NodeProps {
                        transform: Transform::translate(2, 1),
                        opacity: 76,
                        clip: Some(Rect {
                            x: 26,
                            y: 11,
                            width: 98,
                            height: 22,
                        }),
                        ..NodeProps::opaque(3)
                    },
                ),
            ],
        };
        let Some(mut gpu) = dx12_hardware_test_compositor_with_font(Some(font.clone())) else {
            return;
        };
        let mut gpu_decoder = RequestedPatternDecoder::default();
        let gpu_frame = gpu
            .render(&scene, &mut gpu_decoder)
            .expect("DX12 text render");
        let mut cpu = crate::CpuCompositor::new(Some(font));
        let mut cpu_decoder = RequestedPatternDecoder::default();
        let cpu_frame = cpu
            .render(&scene, &mut cpu_decoder)
            .expect("CPU text golden");
        assert_eq!(gpu_decoder.calls, 0);
        assert_eq!(cpu_decoder.calls, 0);
        assert_frames_within_one(&gpu_frame, &cpu_frame);
        let fingerprint = frame_fingerprint(&cpu_frame);
        assert_eq!(
            fingerprint, 0xbeed_c8fc_83d4_8563,
            "fixture-font CPU golden changed: {fingerprint:016x}"
        );
        assert_eq!(
            gpu.text_cache_stats(),
            TextCacheStats {
                hits: 0,
                misses: 2,
                evictions: 0,
                entries: 2,
                bytes: ((title_bounds.width * title_bounds.height
                    + callout_bounds.width * callout_bounds.height)
                    * 4) as usize,
            }
        );
        let second = gpu
            .render(&scene, &mut gpu_decoder)
            .expect("cached text render");
        assert_eq!(second, gpu_frame, "cache hits must preserve the RawFrame");
        assert_eq!(gpu.text_cache_stats().hits, 2);
        assert_eq!(gpu.text_cache_stats().misses, 2);
    }

    #[cfg(windows)]
    #[test]
    fn dx12_matches_cpu_within_one_for_video_image_rotation_clip_and_shapes() {
        let canvas = Canvas {
            width: 40,
            height: 30,
        };
        let scene = RenderScene {
            canvas,
            nodes: vec![
                video_node(canvas),
                RenderNode::Image(ImageNode {
                    props: NodeProps {
                        transform: Transform {
                            translate_x: 1,
                            translate_y: 0,
                            scale_x: 1_000,
                            scale_y: 1_000,
                            rotation_mdeg: 90_000,
                        },
                        opacity: 70,
                        clip: Some(Rect {
                            x: 13,
                            y: 7,
                            width: 5,
                            height: 8,
                        }),
                        z: 1,
                        blend: BlendMode::SrcOver,
                    },
                    bounds: Rect {
                        x: 8,
                        y: 7,
                        width: 12,
                        height: 8,
                    },
                    asset: AssetRef {
                        media_name: "image".into(),
                        locator: MediaLocator::File {
                            path: "image.png".into(),
                        },
                    },
                }),
                RenderNode::Shape(ShapeNode {
                    props: NodeProps {
                        opacity: 50,
                        transform: Transform {
                            translate_x: 2,
                            translate_y: 1,
                            scale_x: 1_500,
                            scale_y: 1_500,
                            rotation_mdeg: 0,
                        },
                        ..NodeProps::opaque(2)
                    },
                    bounds: Rect {
                        x: 20,
                        y: 12,
                        width: 12,
                        height: 8,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::YELLOW,
                }),
                RenderNode::Shape(ShapeNode {
                    props: NodeProps::opaque(3),
                    bounds: Rect {
                        x: 30,
                        y: 22,
                        width: 8,
                        height: 5,
                    },
                    kind: ShapeKind::Rectangle,
                    fill: Rgba::CYAN,
                }),
            ],
        };
        let Some(mut gpu) = dx12_hardware_test_compositor() else {
            return;
        };
        let mut gpu_decoder = RequestedPatternDecoder::default();
        let gpu_frame = gpu.render(&scene, &mut gpu_decoder).expect("DX12 render");
        let mut cpu = crate::CpuCompositor::new(None);
        let mut cpu_decoder = RequestedPatternDecoder::default();
        let cpu_frame = cpu.render(&scene, &mut cpu_decoder).expect("CPU render");
        assert_eq!(gpu_decoder.calls, 2, "video and image must both decode");
        assert_eq!(cpu_decoder.calls, 2, "video and image must both decode");
        assert_frames_within_one(&gpu_frame, &cpu_frame);
        let second = gpu.render(&scene, &mut gpu_decoder).expect("second render");
        assert_eq!(
            gpu.rendered_frames(),
            2,
            "one runtime must serve both frames"
        );
        assert_eq!(gpu_frame, second, "same input must be deterministic");
        assert_eq!(
            gpu.selection().active,
            Some(crate::RendererBackend::GpuDx12)
        );
    }

    #[test]
    fn non_windows_dx12_request_is_typed_and_never_falls_back() {
        if cfg!(windows) {
            return;
        }
        let error = GpuRendererRuntime::new_dx12().err().expect("typed error");
        assert_eq!(error.selection().requested, RendererRequest::RequireGpuDx12);
        assert_eq!(error.selection().active, None);
    }

    #[test]
    fn preview_options_remain_explicit() {
        let options = PreviewOptions::new("out.mp4".into(), ".".into());
        assert_eq!(options.renderer, RendererRequest::RequireCpu);
    }
}
