use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::str::FromStr;

use clap::{Args, Parser, Subcommand, ValueEnum};
use lattice_core::{EditProposal, LocusId, SemanticEdit, Time};
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

#[derive(Debug, Subcommand)]
enum Command {
    /// Parse, compile, and report diagnostics.
    Check { file: PathBuf },
    /// Compile VEL to Core IR.
    Compile {
        file: PathBuf,
        #[arg(long)]
        emit_ir: bool,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Show magic expansions (convention, freeze, title, flow).
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
    /// Propose a semantic title edit. Does not write the VEL file.
    Propose {
        file: PathBuf,
        #[arg(long)]
        locus: Option<String>,
        #[arg(long)]
        title_text: Option<String>,
        #[arg(long)]
        title_at: Option<String>,
        #[arg(long)]
        title_for: Option<String>,
        #[arg(long)]
        title_opacity: Option<u8>,
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
        Command::Propose {
            file,
            locus,
            title_text,
            title_at,
            title_for,
            title_opacity,
        } => propose_command(
            &file,
            locus.as_deref(),
            title_text,
            title_at.as_deref(),
            title_for.as_deref(),
            title_opacity,
            cli.json,
        ),
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
        println!("{}", serde_json::to_string_pretty(&projection)?);
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
    locus: Option<&str>,
    title_text: Option<String>,
    title_at: Option<&str>,
    title_for: Option<&str>,
    title_opacity: Option<u8>,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let engine = Engine::default();
    let compilation = engine.compile_path(file)?;
    let before = compilation.source.clone();
    let target = if let Some(id) = locus {
        engine.inspect(&compilation, &LocusId::new(id))?.locus
    } else {
        engine
            .loci(&compilation)?
            .into_iter()
            .find(|locus| locus.kind == lattice_engine::LocusKind::Title)
            .ok_or("no title locus")?
    };
    let edit = SemanticEdit::Title {
        text: title_text,
        at: title_at.map(parse_cli_time).transpose()?,
        duration: title_for.map(parse_cli_time).transpose()?,
        opacity: title_opacity,
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
