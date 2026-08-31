use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Args, Parser, Subcommand, ValueEnum};
use lattice_core::{
    EditProposal, LocusId, NormalizedPosition, NormalizedScale, SemanticEdit, Time,
};
use lattice_engine::{
    Compilation, Engine, EngineError, ExportError, LocalToneProvider, OutputSpec, PreviewOptions,
    RendererInitError, RendererRequest, ResolveOptions,
};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "lattice", version, about = "Lattice video editing CLI")]
struct Cli {
    /// Machine-readable output. Always available for coding agents.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum RendererArg {
    #[default]
    Cpu,
    GpuDx12,
}

impl From<RendererArg> for RendererRequest {
    fn from(value: RendererArg) -> Self {
        match value {
            RendererArg::Cpu => Self::RequireCpu,
            RendererArg::GpuDx12 => Self::RequireGpuDx12,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameRateArg {
    num: i64,
    den: i64,
}

impl FromStr for FrameRateArg {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (num, den) = match value.split_once('/') {
            Some((num, den)) if !den.contains('/') => (num, den),
            Some(_) => return Err("frame rate must be NUM or NUM/DEN".into()),
            None => (value, "1"),
        };
        let num = num
            .parse::<i64>()
            .map_err(|_| "frame-rate numerator must be an integer".to_string())?;
        let den = den
            .parse::<i64>()
            .map_err(|_| "frame-rate denominator must be an integer".to_string())?;
        if num <= 0 || den <= 0 {
            return Err("frame rate must be greater than zero".into());
        }
        Ok(Self { num, den })
    }
}

fn parse_even_dimension(value: &str) -> Result<u32, String> {
    let dimension = value
        .parse::<u32>()
        .map_err(|_| "dimension must be a positive integer".to_string())?;
    if dimension == 0 {
        return Err("dimension must be greater than zero".into());
    }
    if dimension % 2 != 0 {
        return Err("dimension must be even for yuv420p output".into());
    }
    Ok(dimension)
}

fn parse_fixed_decimal(value: &str, fractional_digits: usize, label: &str) -> Result<u16, String> {
    let value = value.trim().strip_suffix('%').unwrap_or(value.trim());
    if value.starts_with('-') {
        return Err(format!("{label} must not be negative"));
    }
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > fractional_digits
    {
        return Err(format!(
            "{label} must be a decimal with at most {fractional_digits} fractional digit(s)"
        ));
    }
    let whole = whole
        .parse::<u32>()
        .map_err(|_| format!("{label} is out of range"))?;
    let mut fractional = fraction.to_string();
    fractional.extend(std::iter::repeat_n(
        '0',
        fractional_digits.saturating_sub(fractional.len()),
    ));
    let fractional = if fractional.is_empty() {
        0
    } else {
        fractional
            .parse::<u32>()
            .map_err(|_| format!("{label} is out of range"))?
    };
    let factor = 10_u32.pow(u32::try_from(fractional_digits).unwrap_or(0));
    u16::try_from(
        whole
            .checked_mul(factor)
            .and_then(|value| value.checked_add(fractional))
            .ok_or_else(|| format!("{label} is out of range"))?,
    )
    .map_err(|_| format!("{label} is out of range"))
}

fn parse_canvas_percent(value: &str) -> Result<u16, String> {
    let basis_points = parse_fixed_decimal(value, 2, "Canvas position")?;
    NormalizedPosition::new(basis_points, 0)
        .map(|position| position.x)
        .ok_or_else(|| "Canvas position must be between 0% and 100%".to_string())
}

fn parse_scale_percent(value: &str) -> Result<u16, String> {
    let milli = parse_fixed_decimal(value, 1, "overlay scale")?;
    NormalizedScale::new(milli)
        .map(|scale| scale.milli)
        .ok_or_else(|| "overlay scale must be between 25% and 200%".to_string())
}

#[derive(Clone, Copy, Debug, Args)]
struct OutputSpecArgs {
    /// Output width in pixels (must be even).
    #[arg(long, default_value = "320", value_parser = parse_even_dimension)]
    width: u32,
    /// Output height in pixels (must be even).
    #[arg(long, default_value = "180", value_parser = parse_even_dimension)]
    height: u32,
    /// Output frame rate as NUM or NUM/DEN (for example 30 or 30000/1001).
    #[arg(long, default_value = "10", value_name = "NUM[/DEN]")]
    fps: FrameRateArg,
}

impl OutputSpecArgs {
    fn output_spec(self) -> OutputSpec {
        OutputSpec {
            width: self.width,
            height: self.height,
            fps_num: self.fps.num,
            fps_den: self.fps.den,
            ..OutputSpec::preview()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum ProposeEditKind {
    Title,
    Callout,
    Trim,
    Split,
    Delete,
    SetGain,
    SetFade,
    ReorderScene,
    SetPosition,
    ResizeOverlay,
}

#[derive(Debug, Args)]
struct ProposeArgs {
    /// Shared semantic locus to edit. Required except for legacy title/callout discovery.
    #[arg(long)]
    locus: Option<String>,
    /// Semantic edit kind. Existing title/callout flags may still infer their kind.
    #[arg(long, value_enum)]
    edit: Option<ProposeEditKind>,
    #[arg(long)]
    title_text: Option<String>,
    #[arg(long)]
    title_at: Option<String>,
    #[arg(long)]
    title_for: Option<String>,
    #[arg(long)]
    title_opacity: Option<u8>,
    #[arg(long)]
    callout_text: Option<String>,
    #[arg(long)]
    callout_at: Option<String>,
    #[arg(long)]
    callout_for: Option<String>,
    /// New source in-point for a trim.
    #[arg(long)]
    trim_in: Option<String>,
    /// New source out-point for a trim.
    #[arg(long)]
    trim_out: Option<String>,
    /// Source-relative split time.
    #[arg(long)]
    split_at: Option<String>,
    /// Source gain in decibels.
    #[arg(long, allow_hyphen_values = true)]
    gain_db: Option<i32>,
    /// Fade-in duration.
    #[arg(long)]
    fade_in: Option<String>,
    /// Move a scene before this scene name. Omit with reorder-scene to append.
    #[arg(long)]
    before: Option<String>,
    /// Normalized Canvas x coordinate as 0..=100 percent.
    #[arg(long, value_parser = parse_canvas_percent)]
    position_x: Option<u16>,
    /// Normalized Canvas y coordinate as 0..=100 percent.
    #[arg(long, value_parser = parse_canvas_percent)]
    position_y: Option<u16>,
    /// Uniform overlay scale as 25..=200 percent.
    #[arg(long, value_parser = parse_scale_percent)]
    scale: Option<u16>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse, compile, and report diagnostics.
    Check { file: PathBuf },
    /// Canonicalize VEL whitespace and indentation.
    Fmt {
        file: PathBuf,
        /// Report a nonzero exit when formatting would change the file.
        #[arg(long)]
        check: bool,
    },
    /// Compile VEL to Core IR.
    Compile {
        file: PathBuf,
        #[arg(long)]
        emit_ir: bool,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show compile-time expansions and their origins.
    Explain { file: PathBuf },
    /// Flatten the compiled timeline and encode a preview with `FFmpeg`.
    Render {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Required frame renderer. A GPU request never silently falls back to CPU.
        #[arg(long, value_enum, default_value = "cpu")]
        renderer: RendererArg,
        #[command(flatten)]
        spec: OutputSpecArgs,
    },
    /// Alias of `render`.
    Preview {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        /// Required frame renderer. A GPU request never silently falls back to CPU.
        #[arg(long, value_enum, default_value = "cpu")]
        renderer: RendererArg,
        #[command(flatten)]
        spec: OutputSpecArgs,
    },
    /// Project the shared locus (source / Core / timeline).
    Locus {
        file: PathBuf,
        #[arg(long)]
        byte: Option<u32>,
        #[arg(long)]
        node: Option<String>,
        #[arg(long)]
        time: Option<String>,
    },
    /// Inspect a locus for an external agent.
    Inspect {
        file: PathBuf,
        #[arg(long)]
        locus: String,
    },
    /// Propose any Engine `SemanticEdit`. Does not write the VEL file.
    Propose {
        file: PathBuf,
        #[command(flatten)]
        args: Box<ProposeArgs>,
    },
    /// Apply a JSON proposal to the VEL file and recompile.
    Apply {
        file: PathBuf,
        #[arg(long)]
        proposal: PathBuf,
    },
    /// Reject a JSON proposal; VEL is left unchanged.
    Reject {
        file: PathBuf,
        #[arg(long)]
        proposal: PathBuf,
    },
    /// Materialize generated media into a lock. Compile never does this.
    Resolve {
        file: PathBuf,
        #[arg(long)]
        lock: Option<PathBuf>,
        #[arg(long)]
        artifacts: Option<PathBuf>,
    },
    /// Create a VEL project from a real media file (reference in place, no copy).
    Import {
        media: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Alias of `import` that takes a project directory first.
    New {
        dir: PathBuf,
        #[arg(long)]
        media: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match cli.command {
        Command::Check { file } => {
            let compilation = compile_file(&file)?;
            report(cli.json, &file, &compilation, ReportMode::Check, None)?;
            Ok(exit_for(&compilation))
        }
        Command::Fmt { file, check } => format_command(&file, check, cli.json),
        Command::Compile {
            file,
            emit_ir,
            output,
        } => {
            let compilation = compile_file(&file)?;
            if emit_ir {
                let ir = serde_json::to_string_pretty(&compilation.project)?;
                if let Some(path) = &output {
                    std::fs::write(path, ir)?;
                } else if !cli.json {
                    println!("{ir}");
                }
            }
            report(
                cli.json,
                &file,
                &compilation,
                if emit_ir {
                    ReportMode::CompileIr
                } else {
                    ReportMode::Compile
                },
                output.as_deref(),
            )?;
            Ok(exit_for(&compilation))
        }
        Command::Explain { file } => {
            let compilation = compile_file(&file)?;
            report(cli.json, &file, &compilation, ReportMode::Explain, None)?;
            Ok(exit_for(&compilation))
        }
        Command::Render {
            file,
            output,
            renderer,
            spec,
        }
        | Command::Preview {
            file,
            output,
            renderer,
            spec,
        } => render_file(
            &file,
            &output,
            spec.output_spec(),
            renderer.into(),
            cli.json,
        ),
        Command::Locus {
            file,
            byte,
            node,
            time,
        } => locus_command(&file, byte, node.as_deref(), time.as_deref(), cli.json),
        Command::Inspect { file, locus } => inspect_command(&file, &locus, cli.json),
        Command::Propose { file, args } => propose_command(&file, &args, cli.json),
        Command::Apply { file, proposal } => apply_command(&file, &proposal, cli.json),
        Command::Reject { file, proposal } => reject_command(&file, &proposal, cli.json),
        Command::Resolve {
            file,
            lock,
            artifacts,
        } => resolve_command(&file, lock.as_deref(), artifacts.as_deref(), cli.json),
        Command::Import { media, output } => import_command(&media, output.as_deref(), cli.json),
        Command::New { dir, media } => import_command(&media, Some(&dir), cli.json),
    }
}

fn format_command(
    file: &Path,
    check: bool,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(file)?;
    let engine = Engine::default();
    let formatted = engine.format_vel(&source)?;
    let changed = formatted != source;
    let written = changed && !check;
    if written {
        engine.write_source_atomic(file, &formatted)?;
    }
    let ok = !check || !changed;
    if json {
        let payload = serde_json::json!({
            "ok": ok,
            "file": file.display().to_string(),
            "changed": changed,
            "written": written,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else if check && changed {
        println!("needs formatting: {}", file.display());
    } else if written {
        println!("formatted: {}", file.display());
    } else {
        println!("already formatted: {}", file.display());
    }
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}

fn import_command(
    media: &Path,
    output: Option<&Path>,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let imported = engine.import_media(media, output)?;
    let compilation = engine.compile_path(&imported.vel_path)?;
    if json {
        let payload = serde_json::json!({
            "ok": !compilation.has_errors(),
            "file": imported.vel_path.display().to_string(),
            "project_dir": imported.project_dir.display().to_string(),
            "locator": imported.locator,
            "duration": imported.media_info.duration.to_string(),
            "width": imported.media_info.width,
            "height": imported.media_info.height,
            "has_video": imported.media_info.has_video,
            "has_audio": imported.media_info.has_audio,
            "diagnostics": compilation.diagnostics,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "ok: {} (duration {})",
            imported.vel_path.display(),
            imported.media_info.duration
        );
    }
    Ok(exit_for(&compilation))
}

fn render_file(
    file: &Path,
    output: &Path,
    spec: OutputSpec,
    renderer: RendererRequest,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let compilation = compile_file(file)?;
    if compilation.has_errors() {
        report(json, file, &compilation, ReportMode::Check, None)?;
        return Ok(ExitCode::from(1));
    }
    let output = resolve_output(output);
    let media_root = file.parent().unwrap_or_else(|| Path::new("."));
    let engine = Engine::default();
    let lock = resolve_render_lock(&engine, &compilation, media_root)?;
    let report = match engine.render_with_options(
        &compilation.project,
        &PreviewOptions {
            output: output.clone(),
            media_root: media_root.to_path_buf(),
            lock: lock.or_else(|| Engine::load_lock(media_root)),
            spec,
            renderer,
            allow_fixtures: false,
            font: None,
        },
    ) {
        Ok(report) => report,
        Err(error) => {
            if json && let Some(failure) = renderer_failure(&error, renderer) {
                let payload = renderer_failure_json(file, &output, &failure);
                println!("{}", serde_json::to_string_pretty(&payload)?);
                return Ok(ExitCode::from(2));
            }
            return Err(error.into());
        }
    };
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "file": file.display().to_string(),
            "output": report.output,
            "duration": report.duration,
            "spec": report.spec,
            "hold_segments": report.plan.hold_segments,
            "overlays": report.plan.overlays,
            "renderer": report.renderer,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "ok: wrote {} (duration {}, {}x{} @ {}/{} fps, renderer {})",
            report.output.display(),
            report.duration,
            report.spec.width,
            report.spec.height,
            report.spec.fps_num,
            report.spec.fps_den,
            report.renderer
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn resolve_render_lock(
    engine: &Engine,
    compilation: &Compilation,
    media_root: &Path,
) -> Result<Option<lattice_core::ResolveLock>, Box<dyn std::error::Error>> {
    let has_generated = compilation
        .project
        .media
        .iter()
        .any(|media| matches!(media.locator, lattice_core::MediaLocator::Generated { .. }));
    if !has_generated {
        return Ok(None);
    }

    let artifact_dir = media_root.join(".lattice");
    let lock_path = media_root.join("lattice.lock.json");
    let existing = std::fs::read_to_string(&lock_path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    let mut provider = LocalToneProvider;
    let resolution = engine.resolve(
        &compilation.project,
        &ResolveOptions {
            media_root,
            artifact_dir: &artifact_dir,
            lock: existing.as_ref(),
        },
        &mut provider,
    )?;
    std::fs::write(&lock_path, serde_json::to_string_pretty(&resolution.lock)?)?;
    Ok(Some(resolution.lock))
}

struct RendererFailure {
    requested: RendererRequest,
    phase: &'static str,
    kind: &'static str,
    stage: Option<String>,
    reason: String,
}

fn renderer_failure_json(
    file: &Path,
    output: &Path,
    failure: &RendererFailure,
) -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "file": file.display().to_string(),
        "output": output,
        "renderer": {
            "requested": failure.requested,
            "active": serde_json::Value::Null,
            "adapter": serde_json::Value::Null,
            "reason": failure.reason,
        },
        "failure": {
            "phase": failure.phase,
            "kind": failure.kind,
            "stage": failure.stage,
            "reason": failure.reason,
        },
    })
}

fn renderer_failure(error: &EngineError, requested: RendererRequest) -> Option<RendererFailure> {
    let EngineError::Export(export) = error else {
        return None;
    };
    match export {
        ExportError::Renderer(init) => {
            let (kind, stage) = match init {
                RendererInitError::Unavailable { .. } => ("unavailable", None),
                RendererInitError::Initialization { stage, .. } => {
                    ("initialization", Some(stage.to_string()))
                }
            };
            Some(RendererFailure {
                requested: init.selection().requested,
                phase: "initialization",
                kind,
                stage,
                reason: error.to_string(),
            })
        }
        ExportError::RendererRender(render) => Some(RendererFailure {
            requested,
            phase: "render",
            kind: render.kind(),
            stage: None,
            reason: error.to_string(),
        }),
        _ => None,
    }
}

fn resolve_output(output: &Path) -> PathBuf {
    if output.extension().is_none() || output.is_dir() {
        output.join("preview.mp4")
    } else {
        output.to_path_buf()
    }
}

fn compile_file(path: &Path) -> Result<Compilation, Box<dyn std::error::Error>> {
    Ok(Engine::default().compile_path(path)?)
}

fn locus_command(
    file: &Path,
    byte: Option<u32>,
    node: Option<&str>,
    time: Option<&str>,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let compilation = engine.compile_path(file)?;
    let locus = if let Some(node) = node {
        engine.locus_for_node(&compilation, node)?
    } else if let Some(byte) = byte {
        engine.locus_at_source(&compilation, byte)?
    } else if let Some(time) = time {
        engine.locus_at_timeline(&compilation, parse_cli_time(time)?)?
    } else {
        engine
            .loci(&compilation)?
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
    };
    let Some(locus) = locus else {
        return Err("no matching locus".into());
    };
    let projection = engine.inspect(&compilation, &locus.id)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&projection)?);
    } else {
        println!(
            "locus {} ({:?}) node {} label {:?}",
            locus.id.as_str(),
            locus.kind,
            locus.node_id,
            locus.label
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn inspect_command(
    file: &Path,
    locus: &str,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let compilation = engine.compile_path(file)?;
    let projection = engine.inspect(&compilation, &LocusId::new(locus))?;
    if json {
        let mut value = serde_json::to_value(&projection)?;
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "legal".into(),
                serde_json::to_value(engine.legal_edits(&projection.locus))?,
            );
        }
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{}", projection.locus.id.as_str());
        println!("node {}", projection.core.node_id);
        if let Some(source) = &projection.source {
            println!(
                "source {}:{}-{}",
                source.span.line, source.span.start, source.span.end
            );
        }
        if let Some(timeline) = &projection.timeline {
            println!(
                "timeline {} for {}",
                timeline.span.start, timeline.span.duration
            );
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn propose_command(
    file: &Path,
    args: &ProposeArgs,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let compilation = engine.compile_path(file)?;
    let before = compilation.source.clone();
    let (kind, edit) = semantic_edit_from_args(args)?;
    let target = if let Some(id) = args.locus.as_deref() {
        engine.inspect(&compilation, &LocusId::new(id))?.locus
    } else if kind == ProposeEditKind::Callout {
        engine
            .loci(&compilation)?
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Callout)
            .ok_or("no callout locus")?
    } else if kind == ProposeEditKind::Title {
        engine
            .loci(&compilation)?
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
            .ok_or("no title locus")?
    } else {
        return Err(format!("--locus is required for --edit {}", kind.cli_name()).into());
    };
    let proposal = engine.propose(&compilation, &target, edit)?;
    if compilation.source != before {
        return Err("propose mutated current VEL".into());
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&proposal)?);
    } else {
        println!("{}", proposal.description);
        println!("{}", proposal.vel_diff);
    }
    Ok(ExitCode::SUCCESS)
}

impl ProposeEditKind {
    const fn cli_name(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Callout => "callout",
            Self::Trim => "trim",
            Self::Split => "split",
            Self::Delete => "delete",
            Self::SetGain => "set-gain",
            Self::SetFade => "set-fade",
            Self::ReorderScene => "reorder-scene",
            Self::SetPosition => "set-position",
            Self::ResizeOverlay => "resize-overlay",
        }
    }
}

fn semantic_edit_from_args(
    args: &ProposeArgs,
) -> Result<(ProposeEditKind, SemanticEdit), Box<dyn std::error::Error>> {
    let has_title = args.title_text.is_some()
        || args.title_at.is_some()
        || args.title_for.is_some()
        || args.title_opacity.is_some();
    let has_callout =
        args.callout_text.is_some() || args.callout_at.is_some() || args.callout_for.is_some();
    let has_trim = args.trim_in.is_some() || args.trim_out.is_some();
    let kind = propose_edit_kind(args, has_title, has_callout, has_trim)?;

    let edit = match kind {
        ProposeEditKind::Title if has_title => SemanticEdit::Title {
            text: args.title_text.clone(),
            at: args.title_at.as_deref().map(parse_cli_time).transpose()?,
            duration: args.title_for.as_deref().map(parse_cli_time).transpose()?,
            opacity: args.title_opacity,
        },
        ProposeEditKind::Callout if has_callout => SemanticEdit::Callout {
            text: args.callout_text.clone(),
            at: args.callout_at.as_deref().map(parse_cli_time).transpose()?,
            duration: args
                .callout_for
                .as_deref()
                .map(parse_cli_time)
                .transpose()?,
        },
        ProposeEditKind::Trim if has_trim => SemanticEdit::Trim {
            in_point: args.trim_in.as_deref().map(parse_cli_time).transpose()?,
            out_point: args.trim_out.as_deref().map(parse_cli_time).transpose()?,
        },
        ProposeEditKind::Split => SemanticEdit::Split {
            at: parse_cli_time(args.split_at.as_deref().ok_or("split needs --split-at")?)?,
        },
        ProposeEditKind::Delete => SemanticEdit::Delete,
        ProposeEditKind::SetGain => SemanticEdit::SetGain {
            db: args.gain_db.ok_or("set-gain needs --gain-db")?,
        },
        ProposeEditKind::SetFade => SemanticEdit::SetFade {
            fade_in: Some(parse_cli_time(
                args.fade_in.as_deref().ok_or("set-fade needs --fade-in")?,
            )?),
        },
        ProposeEditKind::ReorderScene => SemanticEdit::ReorderScene {
            before: args.before.clone(),
        },
        ProposeEditKind::SetPosition => SemanticEdit::SetPosition {
            position: propose_position(args)?,
        },
        ProposeEditKind::ResizeOverlay => SemanticEdit::ResizeOverlay {
            position: propose_position(args)?,
            scale: NormalizedScale::new(args.scale.ok_or("resize-overlay needs --scale")?)
                .ok_or("overlay scale is out of range")?,
        },
        ProposeEditKind::Title => return Err("title needs a --title-* field".into()),
        ProposeEditKind::Callout => return Err("callout needs a --callout-* field".into()),
        ProposeEditKind::Trim => return Err("trim needs --trim-in or --trim-out".into()),
    };
    Ok((kind, edit))
}

fn propose_edit_kind(
    args: &ProposeArgs,
    has_title: bool,
    has_callout: bool,
    has_trim: bool,
) -> Result<ProposeEditKind, Box<dyn std::error::Error>> {
    let has_position = args.position_x.is_some() || args.position_y.is_some();
    let inferred_groups = [
        (ProposeEditKind::Title, has_title),
        (ProposeEditKind::Callout, has_callout),
        (ProposeEditKind::Trim, has_trim),
        (ProposeEditKind::Split, args.split_at.is_some()),
        (ProposeEditKind::SetGain, args.gain_db.is_some()),
        (ProposeEditKind::SetFade, args.fade_in.is_some()),
        (ProposeEditKind::ReorderScene, args.before.is_some()),
        (
            if args.scale.is_some() {
                ProposeEditKind::ResizeOverlay
            } else {
                ProposeEditKind::SetPosition
            },
            has_position || args.scale.is_some(),
        ),
    ]
    .into_iter()
    .filter_map(|(kind, present)| present.then_some(kind))
    .collect::<Vec<_>>();

    let kind = match args.edit {
        Some(kind) => {
            if inferred_groups.iter().any(|field_kind| {
                *field_kind != kind
                    && !matches!(
                        (*field_kind, kind),
                        (ProposeEditKind::SetPosition, ProposeEditKind::ResizeOverlay)
                    )
            }) {
                let fields = inferred_groups
                    .iter()
                    .map(|kind| kind.cli_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!(
                    "flags for {fields} cannot be combined with --edit {}",
                    kind.cli_name()
                )
                .into());
            }
            kind
        }
        None => match inferred_groups.as_slice() {
            [kind] => *kind,
            [] => ProposeEditKind::Title,
            _ => return Err("propose flags name more than one semantic edit kind".into()),
        },
    };
    Ok(kind)
}

fn propose_position(args: &ProposeArgs) -> Result<NormalizedPosition, Box<dyn std::error::Error>> {
    let x = args.position_x.ok_or("Canvas edit needs --position-x")?;
    let y = args.position_y.ok_or("Canvas edit needs --position-y")?;
    NormalizedPosition::new(x, y).ok_or_else(|| "Canvas position is out of range".into())
}

fn apply_command(
    file: &Path,
    proposal_path: &Path,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let compilation = engine.compile_path(file)?;
    let proposal: EditProposal = serde_json::from_str(&std::fs::read_to_string(proposal_path)?)?;
    let applied = match engine.apply_proposal(&compilation.source, &proposal) {
        Ok(source) => source,
        Err(err) => {
            if let Some((expected, found)) = err.stale_revisions() {
                if json {
                    let payload = serde_json::json!({
                        "ok": false,
                        "applied": false,
                        "error": {
                            "code": "LAT-EDIT-STALE",
                            "message": err.to_string(),
                            "expected": expected,
                            "found": found,
                        },
                        "file": file.display().to_string(),
                    });
                    println!("{}", serde_json::to_string_pretty(&payload)?);
                } else {
                    eprintln!("{err}");
                }
                return Ok(ExitCode::from(1));
            }
            return Err(err.into());
        }
    };
    engine.write_source_atomic(file, &applied)?;
    let recompiled = engine.compile_path(file)?;
    if json {
        let payload = serde_json::json!({
            "ok": !recompiled.has_errors(),
            "applied": true,
            "file": file.display().to_string(),
            "description": proposal.description,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("applied: {}", proposal.description);
    }
    Ok(exit_for(&recompiled))
}

fn reject_command(
    file: &Path,
    proposal_path: &Path,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let before = std::fs::read(file)?;
    let proposal: EditProposal = serde_json::from_str(&std::fs::read_to_string(proposal_path)?)?;
    let engine = Engine::default();
    let compilation = engine.compile_path(file)?;
    let rejected = engine.reject_proposal(&compilation.source, &proposal);
    let after = std::fs::read(file)?;
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "applied": false,
            "unchanged": before == after && rejected.as_bytes() == before,
            "file": file.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!("rejected; VEL unchanged");
    }
    Ok(ExitCode::SUCCESS)
}

fn resolve_command(
    file: &Path,
    lock_path: Option<&Path>,
    artifacts: Option<&Path>,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let compilation = engine.compile_path(file)?;
    let media_root = file.parent().unwrap_or_else(|| Path::new("."));
    let artifact_dir = artifacts.map_or_else(|| media_root.join(".lattice"), Path::to_path_buf);
    let existing = match lock_path {
        Some(path) if path.is_file() => {
            Some(serde_json::from_str(&std::fs::read_to_string(path)?)?)
        }
        _ => None,
    };
    let mut provider = LocalToneProvider;
    let resolution = engine.resolve(
        &compilation.project,
        &ResolveOptions {
            media_root,
            artifact_dir: &artifact_dir,
            lock: existing.as_ref(),
        },
        &mut provider,
    )?;
    let out_lock =
        lock_path.map_or_else(|| media_root.join("lattice.lock.json"), Path::to_path_buf);
    std::fs::write(&out_lock, serde_json::to_string_pretty(&resolution.lock)?)?;
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "lock": out_lock.display().to_string(),
            "provider_calls": resolution.provider_calls,
            "assets": resolution.assets,
            "diagnostics": resolution.diagnostics,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "ok: lock {} ({} provider call{})",
            out_lock.display(),
            resolution.provider_calls,
            if resolution.provider_calls == 1 {
                ""
            } else {
                "s"
            }
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn parse_cli_time(text: &str) -> Result<Time, Box<dyn std::error::Error>> {
    let trimmed = text.trim().trim_end_matches('s');
    if let Some((whole, frac)) = trimmed.split_once('.') {
        let whole: i64 = whole.parse()?;
        let frac: i64 = frac.parse()?;
        let digits = u32::try_from(frac.to_string().len())?;
        Ok(Time::from_decimal_seconds(whole, frac, digits)?)
    } else {
        Ok(Time::seconds(trimmed.parse()?))
    }
}

#[derive(Clone, Copy)]
enum ReportMode {
    Check,
    Compile,
    CompileIr,
    Explain,
}

#[derive(Serialize)]
struct JsonReport<'a> {
    ok: bool,
    file: String,
    diagnostics: &'a [lattice_core::Diagnostic],
    explain: &'a [lattice_engine::ExplainEvent],
    #[serde(skip_serializing_if = "Option::is_none")]
    ir: Option<&'a lattice_core::Project>,
}

fn report(
    json: bool,
    file: &Path,
    compilation: &Compilation,
    mode: ReportMode,
    _output: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        let payload = JsonReport {
            ok: !compilation.has_errors(),
            file: file.display().to_string(),
            diagnostics: &compilation.diagnostics,
            explain: &compilation.explain,
            ir: matches!(mode, ReportMode::CompileIr).then_some(&compilation.project),
        };
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }
    for diagnostic in &compilation.diagnostics {
        let where_ = diagnostic
            .span
            .map(|span| format!("{}:{}: ", span.line, span.column))
            .unwrap_or_default();
        println!(
            "{where_}{:?} {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        );
    }
    match mode {
        ReportMode::Explain => {
            for event in &compilation.explain {
                println!("- {}", event.message);
            }
        }
        ReportMode::Check | ReportMode::Compile if !compilation.has_errors() => {
            println!(
                "ok: project `{}` ({} scene{})",
                compilation.project.name,
                compilation.project.scenes.len(),
                if compilation.project.scenes.len() == 1 {
                    ""
                } else {
                    "s"
                }
            );
        }
        _ => {}
    }
    Ok(())
}

fn exit_for(compilation: &Compilation) -> ExitCode {
    if compilation.has_errors() {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_engine::{RendererInitStage, RendererRenderError, RendererSelection};

    #[test]
    fn parses_integer_and_fractional_frame_rates() {
        assert_eq!(
            "30".parse::<FrameRateArg>().unwrap(),
            FrameRateArg { num: 30, den: 1 }
        );
        assert_eq!(
            "30000/1001".parse::<FrameRateArg>().unwrap(),
            FrameRateArg {
                num: 30_000,
                den: 1001
            }
        );
        for invalid in ["0", "30/0", "-1", "30/1/2", "x"] {
            assert!(invalid.parse::<FrameRateArg>().is_err(), "{invalid}");
        }
    }

    #[test]
    fn classifies_renderer_init_and_render_failures_for_json() {
        let request = RendererRequest::RequireGpuDx12;
        let init = EngineError::Export(ExportError::Renderer(RendererInitError::Unavailable {
            selection: RendererSelection {
                requested: request,
                active: None,
                adapter: None,
                reason: "no adapter".into(),
            },
        }));
        let init = renderer_failure(&init, request).expect("init classification");
        assert_eq!(init.requested, request);
        assert_eq!(init.phase, "initialization");
        assert_eq!(init.kind, "unavailable");
        assert_eq!(init.stage, None);
        assert!(init.reason.contains("no adapter"));

        let staged =
            EngineError::Export(ExportError::Renderer(RendererInitError::Initialization {
                selection: RendererSelection {
                    requested: request,
                    active: None,
                    adapter: None,
                    reason: "device creation failed".into(),
                },
                stage: RendererInitStage::Device,
                message: "lost device".into(),
            }));
        let staged = renderer_failure(&staged, request).expect("staged init classification");
        assert_eq!(staged.kind, "initialization");
        assert_eq!(staged.stage.as_deref(), Some("device"));

        let render = EngineError::Export(ExportError::RendererRender(
            RendererRenderError::DeviceLost {
                reason: "destroyed".into(),
                message: "queue submission failed".into(),
            },
        ));
        let render = renderer_failure(&render, request).expect("render classification");
        assert_eq!(render.requested, request);
        assert_eq!(render.phase, "render");
        assert_eq!(render.kind, "device_lost");
        assert_eq!(render.stage, None);
        assert!(render.reason.contains("destroyed"));

        let payload = renderer_failure_json(Path::new("main.vel"), Path::new("out.mp4"), &render);
        assert_eq!(payload["ok"], false);
        assert_eq!(payload["renderer"]["requested"], "require_gpu_dx12");
        assert!(payload["renderer"]["active"].is_null());
        assert!(payload["renderer"]["adapter"].is_null());
        assert_eq!(payload["failure"]["phase"], "render");
        assert_eq!(payload["failure"]["kind"], "device_lost");
        assert!(
            payload["failure"]["reason"]
                .as_str()
                .is_some_and(|reason| reason.contains("destroyed"))
        );
    }
}
