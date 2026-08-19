use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use lattice_engine::{Compilation, Engine};
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
    },
    /// Alias of `render`.
    Preview {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
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
        Command::Render { file, output } | Command::Preview { file, output } => {
            render_file(&file, &output, cli.json)
        }
    }
}

fn render_file(
    file: &Path,
    output: &Path,
    json: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let compilation = compile_file(file)?;
    if compilation.has_errors() {
        report(json, file, &compilation, ReportMode::Check, None)?;
        return Ok(ExitCode::from(1));
    }
    let output = resolve_output(output);
    let media_root = file.parent().unwrap_or_else(|| Path::new("."));
    let report = Engine::default().render(&compilation.project, &output, media_root)?;
    if json {
        let payload = serde_json::json!({
            "ok": true,
            "file": file.display().to_string(),
            "output": report.output,
            "duration": report.duration,
            "hold_segments": report.plan.hold_segments,
            "overlays": report.plan.overlays,
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
    } else {
        println!(
            "ok: wrote {} (duration {})",
            report.output.display(),
            report.duration
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn resolve_output(output: &Path) -> PathBuf {
    if output.extension().is_none() || output.is_dir() {
        output.join("preview.mp4")
    } else {
        output.to_path_buf()
    }
}

fn compile_file(path: &Path) -> Result<Compilation, Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string(path)?;
    Ok(Engine::default().compile(&source)?)
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
