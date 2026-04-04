mod analyze;
mod animate;
mod check;
mod custom_rules;
mod diff;
mod export;
mod generate;
mod import;
mod layout;
mod lsp;
mod mcp;
mod model;
mod parser;
mod preprocess;
mod render;
mod serve;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

// ─── CLI Definition ──────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "forge",
    version,
    about = "A unified software modeling tool — structure, process, and deployment from a single DSL"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse .forge files and render SVG diagrams
    Build {
        /// Input .forge file
        #[arg(short, long, default_value = "forge.forge")]
        source: PathBuf,

        /// Render a specific view by key (default: all views)
        #[arg(long)]
        view: Option<String>,

        /// Output directory for SVG files
        #[arg(short, long, default_value = "_site/diagrams")]
        out: PathBuf,

        /// Rendering style: filled or outline
        #[arg(long, default_value = "filled")]
        style: String,
    },

    /// Lint and validate a model against architectural rules
    Check {
        /// Input .forge file
        #[arg(short, long, default_value = "forge.forge")]
        source: PathBuf,

        /// Minimum severity to report: error, warning, info
        #[arg(long, default_value = "warning")]
        severity: String,

        /// Output format: text, json, sarif
        #[arg(short, long, default_value = "text")]
        format: String,

        /// Custom rules file (.forge-rules)
        #[arg(long)]
        rules: Option<PathBuf>,
    },

    /// Scan codebases and produce a .forge model
    Analyze {
        /// Directories to scan
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Output .forge file
        #[arg(short, long, default_value = "forge.forge")]
        out: PathBuf,

        /// Comma-separated scanner list (code,source,ci,docker,git,k8s,infra)
        #[arg(long, default_value = "code,source,ci,docker,git,k8s,infra")]
        scanners: String,

        /// Exclude directory names (repeatable)
        #[arg(long)]
        exclude: Vec<String>,

        /// Print output to stdout without writing file
        #[arg(long)]
        dry_run: bool,
    },

    /// Generate a static documentation website
    Generate {
        /// Input .forge file
        #[arg(short, long, default_value = "forge.forge")]
        source: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "_site")]
        out: PathBuf,

        /// Site title (defaults to model name)
        #[arg(long)]
        title: Option<String>,

        /// Base URL for deployment (e.g. /repo-name/ for GitHub Pages)
        #[arg(long, default_value = "/")]
        base_url: String,

        /// Diagram rendering style: filled or outline
        #[arg(long, default_value = "filled")]
        style: String,

        /// Baseline .forge file to diff against (highlights changes)
        #[arg(long)]
        baseline: Option<PathBuf>,
    },

    /// Export model as JSON or YAML
    Export {
        /// Input .forge file
        #[arg(short, long, default_value = "forge.forge")]
        source: PathBuf,

        /// Output format: json, yaml
        #[arg(short, long, default_value = "json")]
        format: String,

        /// Output file (defaults to stdout)
        #[arg(short, long)]
        out: Option<PathBuf>,
    },

    /// Import from PlantUML C4 or Mermaid to .forge
    Import {
        /// Input file (.puml, .mmd, or any text)
        #[arg(short, long)]
        source: PathBuf,

        /// Output .forge file (defaults to stdout)
        #[arg(short, long)]
        out: Option<PathBuf>,
    },

    /// Watch for changes and rebuild automatically
    Watch {
        /// Input .forge file
        #[arg(short, long, default_value = "forge.forge")]
        source: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "_site")]
        out: PathBuf,

        /// Diagram rendering style
        #[arg(long, default_value = "filled")]
        style: String,

        /// Baseline .forge file for diff highlighting
        #[arg(long)]
        baseline: Option<PathBuf>,
    },

    /// Start a local preview server with live reload
    Serve {
        /// Input .forge file
        #[arg(short, long, default_value = "forge.forge")]
        source: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "_site")]
        out: PathBuf,

        /// Diagram rendering style
        #[arg(long, default_value = "filled")]
        style: String,

        /// HTTP port
        #[arg(short, long, default_value = "4000")]
        port: u16,

        /// Baseline .forge file for diff highlighting
        #[arg(long)]
        baseline: Option<PathBuf>,

        /// Presentation mode for animated views
        #[arg(long)]
        present: bool,
    },

    /// Start the MCP server for AI agent integration (stdio)
    Mcp {
        /// Input .forge file
        #[arg(short, long, default_value = "forge.forge")]
        source: PathBuf,
    },

    /// Start the Language Server Protocol server (stdio)
    Lsp,
}

// ─── Command Dispatch ────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            source,
            view,
            out,
            style,
        } => cmd_build(&source, view.as_deref(), &out, &style),
        Commands::Check {
            source,
            severity,
            format,
            rules,
        } => cmd_check(&source, &severity, &format, rules.as_deref()),
        Commands::Analyze {
            paths,
            out,
            scanners,
            exclude,
            dry_run,
        } => cmd_analyze(paths, &out, &scanners, exclude, dry_run),
        Commands::Generate {
            source,
            out,
            title,
            base_url,
            style,
            baseline,
        } => cmd_generate(&source, &out, title, &base_url, &style, baseline.as_deref()),
        Commands::Export {
            source,
            format,
            out,
        } => cmd_export(&source, &format, out.as_deref()),
        Commands::Import { source, out } => cmd_import(&source, out.as_deref()),
        Commands::Watch {
            source,
            out,
            style,
            baseline,
        } => cmd_watch(source, out, style, baseline),
        Commands::Serve {
            source,
            out,
            style,
            port,
            baseline,
            present,
        } => cmd_serve(source, out, style, port, baseline, present),
        Commands::Mcp { source } => mcp::run(source),
        Commands::Lsp => {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime")
                .block_on(lsp::run());
        }
    }
}

// ─── Command Handlers ────────────────────────────────────────────

fn cmd_build(source: &Path, view: Option<&str>, out: &Path, style: &str) {
    if style != "filled" && style != "outline" {
        die("--style must be 'filled' or 'outline'");
    }

    let model = load_model(source);
    eprintln!("Parsed model: \"{}\"", model.name);
    eprintln!("  Elements: {}", model.elements.len());
    eprintln!("  Relationships: {}", model.relationships.len());
    eprintln!("  Views: {}", model.views.len());

    fs::create_dir_all(out).unwrap_or_else(|e| die(&format!("creating output dir: {}", e)));

    for v in &model.views {
        if let Some(filter) = view {
            if v.key != filter {
                continue;
            }
        }

        let lo = layout::compute_layout(&model, v);
        let mut svg = render::render_svg(&lo, style);
        if !v.animation.is_empty() {
            svg = animate::animate_svg(&svg, v, &model);
        }

        let path = out.join(format!("{}.svg", v.key));
        fs::write(&path, &svg)
            .unwrap_or_else(|e| die(&format!("writing {}: {}", path.display(), e)));
        eprintln!("  Wrote: {}", path.display());
    }
    eprintln!("Done.");
}

fn cmd_check(source: &Path, severity: &str, format: &str, rules: Option<&Path>) {
    let min_severity = check::Severity::from_str(severity)
        .unwrap_or_else(|| die("--severity must be 'error', 'warning', or 'info'"));

    let model = load_model(source);
    let mut violations = check::check(&model, min_severity);

    if let Some(rules_path) = rules {
        let rules_text = fs::read_to_string(rules_path)
            .unwrap_or_else(|e| die(&format!("reading rules: {}: {}", rules_path.display(), e)));
        let custom = custom_rules::parse_rules(&rules_text)
            .unwrap_or_else(|e| die(&format!("parsing rules: {}", e)));
        let mut cv = custom_rules::evaluate_rules(&custom, &model);
        cv.retain(|v| v.severity >= min_severity);
        violations.extend(cv);
    }

    match format {
        "json" => print_json(&violations),
        "sarif" => print_sarif(&violations, source),
        _ => print_text(&violations),
    }

    if violations
        .iter()
        .any(|v| v.severity == check::Severity::Error)
    {
        process::exit(1);
    }
}

fn cmd_analyze(
    paths: Vec<PathBuf>,
    out: &Path,
    scanners: &str,
    exclude: Vec<String>,
    dry_run: bool,
) {
    let scanner_list: Vec<String> = scanners.split(',').map(|s| s.trim().to_string()).collect();
    let mut config = analyze::AnalyzeConfig {
        paths,
        scanners: scanner_list,
        out: out.to_path_buf(),
        dry_run,
        ..Default::default()
    };
    config.exclude.extend(exclude);

    eprintln!("Scanning...");
    let model = analyze::analyze(&config);
    eprintln!("  Elements: {}", model.elements.len());
    eprintln!("  Relationships: {}", model.relationships.len());

    let forge_text = analyze::emit::emit(&model);
    if dry_run {
        println!("{}", forge_text);
    } else {
        fs::write(out, &forge_text)
            .unwrap_or_else(|e| die(&format!("writing {}: {}", out.display(), e)));
        eprintln!("  Wrote: {}", out.display());
    }
    eprintln!("Done.");
}

fn cmd_generate(
    source: &Path,
    out: &Path,
    title: Option<String>,
    base_url: &str,
    style: &str,
    baseline: Option<&Path>,
) {
    let model = load_model(source);

    let diff_result = baseline.map(|bp| {
        let bm = load_model(bp);
        let dr = diff::diff(&bm, &model);
        eprintln!(
            "Diff: {} added, {} modified, {} removed",
            dr.added_count(),
            dr.modified_count(),
            dr.removed_count()
        );
        dr
    });

    eprintln!("Generating site from \"{}\"...", model.name);
    let source_dir = source.parent().unwrap_or(Path::new(".")).to_path_buf();
    let config = generate::GenerateConfig {
        out_dir: out.to_path_buf(),
        title,
        base_url: base_url.into(),
        style: style.into(),
        source_dir,
    };

    match generate::generate(&model, &config, diff_result.as_ref()) {
        Ok(report) => {
            eprintln!(
                "  {} pages, {} diagrams → {}",
                report.pages,
                report.diagrams,
                out.display()
            );
            eprintln!("Done.");
        }
        Err(e) => die(&format!("generate: {}", e)),
    }
}

fn cmd_export(source: &Path, format: &str, out: Option<&Path>) {
    let model = load_model(source);
    let output = match format {
        "json" => export::to_json(&model),
        "yaml" | "yml" => export::to_yaml(&model),
        _ => die("--format must be 'json' or 'yaml'"),
    };
    write_or_stdout(out, &output);
}

fn cmd_import(source: &Path, out: Option<&Path>) {
    let text =
        fs::read_to_string(source).unwrap_or_else(|e| die(&format!("{}: {}", source.display(), e)));
    let forge_text =
        import::import_to_forge(&text).unwrap_or_else(|e| die(&format!("import: {}", e)));
    write_or_stdout(out, &forge_text);
}

fn cmd_watch(source: PathBuf, out: PathBuf, style: String, baseline: Option<PathBuf>) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    rt.block_on(serve::run_watch(source, out, style, baseline));
}

fn cmd_serve(
    source: PathBuf,
    out: PathBuf,
    style: String,
    port: u16,
    baseline: Option<PathBuf>,
    present: bool,
) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    if present {
        eprintln!("Presentation mode: http://localhost:{}/present.html", port);
    }
    rt.block_on(serve::run_serve(
        source, out, style, baseline, port, present,
    ));
}

// ─── Output Helpers ──────────────────────────────────────────────

fn print_text(violations: &[check::Violation]) {
    if violations.is_empty() {
        eprintln!("No issues found.");
        return;
    }
    eprintln!("Found {} issue(s):\n", violations.len());
    for v in violations {
        println!("{}", v);
    }
}

fn print_json(violations: &[check::Violation]) {
    let results: Vec<serde_json::Value> = violations
        .iter()
        .map(|v| {
            serde_json::json!({
                "rule": v.rule,
                "severity": v.severity.to_string(),
                "element": v.element_id,
                "message": v.message,
            })
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&results).unwrap_or_default()
    );
}

fn print_sarif(violations: &[check::Violation], source: &Path) {
    use serde_json::json;
    let results: Vec<serde_json::Value> = violations
        .iter()
        .map(|v| {
            let level = match v.severity {
                check::Severity::Error => "error",
                check::Severity::Warning => "warning",
                check::Severity::Info => "note",
            };
            json!({
                "ruleId": v.rule,
                "level": level,
                "message": { "text": v.message },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": source.display().to_string() },
                        "region": { "startLine": 1 }
                    },
                    "logicalLocations": v.element_id.as_ref().map(|id| vec![json!({
                        "name": id, "kind": "element"
                    })]).unwrap_or_default(),
                }]
            })
        })
        .collect();

    let sarif = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "forge",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": violations.iter().map(|v| json!({
                        "id": v.rule, "shortDescription": { "text": v.rule },
                    })).collect::<Vec<_>>(),
                }
            },
            "results": results,
        }]
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&sarif).unwrap_or_default()
    );
}

// ─── Utilities ───────────────────────────────────────────────────

fn load_model(source: &Path) -> model::Model {
    let text =
        fs::read_to_string(source).unwrap_or_else(|e| die(&format!("{}: {}", source.display(), e)));
    let base_dir = source.parent().unwrap_or(Path::new("."));
    parser::parse_with_preprocess(&text, base_dir).unwrap_or_else(|e| die(&format!("{}", e)))
}

fn write_or_stdout(out: Option<&Path>, content: &str) {
    if let Some(path) = out {
        fs::write(path, content)
            .unwrap_or_else(|e| die(&format!("writing {}: {}", path.display(), e)));
        eprintln!("Wrote: {}", path.display());
    } else {
        println!("{}", content);
    }
}

fn die(msg: &str) -> ! {
    eprintln!("Error: {}", msg);
    process::exit(1);
}
