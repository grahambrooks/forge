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

#[derive(Parser)]
#[command(name = "forge", about = "A unified software modeling tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse .forge files and render SVG diagrams
    Build {
        /// Input .forge file
        #[arg(long, default_value = "forge.forge")]
        source: PathBuf,

        /// Render a specific view (default: all)
        #[arg(long)]
        view: Option<String>,

        /// Output directory
        #[arg(long, default_value = "forge-output")]
        out: PathBuf,

        /// Rendering style: 'filled' or 'outline'
        #[arg(long, default_value = "filled")]
        style: String,
    },
    /// Scan codebases and produce a .forge model
    Analyze {
        /// Directories to scan (default: current directory)
        #[arg(default_value = ".")]
        paths: Vec<PathBuf>,

        /// Output .forge file
        #[arg(long, default_value = "forge.forge")]
        out: PathBuf,

        /// Comma-separated scanner list: code, ci, docker
        #[arg(long, default_value = "code,ci,docker")]
        scanners: String,

        /// Exclude paths matching these names (repeatable)
        #[arg(long)]
        exclude: Vec<String>,

        /// Show what would be generated without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Generate a static documentation website from a model
    Generate {
        /// Input .forge file
        #[arg(long, default_value = "forge.forge")]
        source: PathBuf,

        /// Output directory
        #[arg(long, default_value = "_site")]
        out: PathBuf,

        /// Site title (default: model name)
        #[arg(long)]
        title: Option<String>,

        /// Base URL for deployment
        #[arg(long, default_value = "/")]
        base_url: String,

        /// Diagram rendering style: 'filled' or 'outline'
        #[arg(long, default_value = "filled")]
        style: String,

        /// Baseline .forge file to diff against (highlights changes)
        #[arg(long)]
        baseline: Option<PathBuf>,
    },
    /// Lint and validate a model against architectural rules
    Check {
        /// Input .forge file
        #[arg(long, default_value = "forge.forge")]
        source: PathBuf,

        /// Minimum severity to report: error, warning, info
        #[arg(long, default_value = "warning")]
        severity: String,

        /// Output format: text, json
        #[arg(long, default_value = "text")]
        format: String,

        /// Custom rules file (.forge-rules)
        #[arg(long)]
        rules: Option<PathBuf>,
    },
    /// Watch for changes and rebuild automatically
    Watch {
        /// Input .forge file
        #[arg(long, default_value = "forge.forge")]
        source: PathBuf,

        /// Output directory
        #[arg(long, default_value = "_site")]
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
        #[arg(long, default_value = "forge.forge")]
        source: PathBuf,

        /// Output directory
        #[arg(long, default_value = "_site")]
        out: PathBuf,

        /// Diagram rendering style
        #[arg(long, default_value = "filled")]
        style: String,

        /// HTTP port
        #[arg(long, default_value = "4000")]
        port: u16,

        /// Baseline .forge file for diff highlighting
        #[arg(long)]
        baseline: Option<PathBuf>,

        /// Presentation mode for animated views (fullscreen, keyboard nav)
        #[arg(long)]
        present: bool,
    },
    /// Export model as JSON or YAML
    Export {
        /// Input .forge file
        #[arg(long, default_value = "forge.forge")]
        source: PathBuf,

        /// Output format: json, yaml
        #[arg(long, default_value = "json")]
        format: String,

        /// Output file (default: stdout)
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Import from PlantUML C4 or Mermaid to .forge
    Import {
        /// Input file (PlantUML .puml or Mermaid .mmd)
        source: PathBuf,

        /// Output .forge file (default: stdout)
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Start the MCP server for AI agent integration (stdio)
    Mcp {
        /// Input .forge file
        #[arg(long, default_value = "forge.forge")]
        source: PathBuf,
    },
    /// Start the Language Server Protocol server (stdio)
    Lsp,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            source,
            view,
            out,
            style,
        } => {
            if style != "filled" && style != "outline" {
                eprintln!("Error: --style must be 'filled' or 'outline'");
                process::exit(1);
            }

            let model = load_model(&source);

            eprintln!("Parsed model: \"{}\"", model.name);
            eprintln!("  Elements: {}", model.elements.len());
            eprintln!("  Relationships: {}", model.relationships.len());
            eprintln!("  Views: {}", model.views.len());

            fs::create_dir_all(&out).unwrap_or_else(|e| {
                eprintln!("Error creating output dir: {}", e);
                process::exit(1);
            });

            for v in &model.views {
                if let Some(ref filter) = view {
                    if &v.key != filter {
                        continue;
                    }
                }

                let lo = layout::compute_layout(&model, v);
                let mut svg = render::render_svg(&lo, &style);

                // Apply animation if the view has frames
                if !v.animation.is_empty() {
                    svg = animate::animate_svg(&svg, v, &model);
                }

                let path = out.join(format!("{}.svg", v.key));
                fs::write(&path, &svg).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", path.display(), e);
                    process::exit(1);
                });
                eprintln!("  Wrote: {}", path.display());
            }

            eprintln!("Done.");
        }
        Commands::Analyze {
            paths,
            out,
            scanners,
            exclude,
            dry_run,
        } => {
            let scanner_list: Vec<String> =
                scanners.split(',').map(|s| s.trim().to_string()).collect();

            let mut config = analyze::AnalyzeConfig {
                paths,
                scanners: scanner_list,
                out: out.clone(),
                dry_run,
                ..Default::default()
            };
            for e in exclude {
                config.exclude.push(e);
            }

            eprintln!("Scanning...");
            let model = analyze::analyze(&config);

            eprintln!("  Elements: {}", model.elements.len());
            eprintln!("  Relationships: {}", model.relationships.len());

            let forge_text = analyze::emit::emit(&model);

            if dry_run {
                println!("{}", forge_text);
            } else {
                fs::write(&out, &forge_text).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", out.display(), e);
                    process::exit(1);
                });
                eprintln!("  Wrote: {}", out.display());
            }
            eprintln!("Done.");
        }
        Commands::Generate {
            source,
            out,
            title,
            base_url,
            style,
            baseline,
        } => {
            let model = load_model(&source);

            // Load and diff baseline if provided
            let diff_result = if let Some(ref baseline_path) = baseline {
                let baseline_model = load_model(baseline_path);
                let dr = diff::diff(&baseline_model, &model);
                eprintln!(
                    "Diff: {} added, {} modified, {} removed",
                    dr.added_count(),
                    dr.modified_count(),
                    dr.removed_count()
                );
                Some(dr)
            } else {
                None
            };

            eprintln!("Generating site from \"{}\"...", model.name);

            let source_dir = source
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from("."));

            let config = generate::GenerateConfig {
                out_dir: out.clone(),
                title,
                base_url,
                style,
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
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            }
        }
        Commands::Check {
            source,
            severity,
            format,
            rules,
        } => {
            let min_severity = match check::Severity::from_str(&severity) {
                Some(s) => s,
                None => {
                    eprintln!("Error: --severity must be 'error', 'warning', or 'info'");
                    process::exit(1);
                }
            };

            let model = load_model(&source);

            let mut violations = check::check(&model, min_severity);

            // Apply custom rules if provided
            if let Some(ref rules_path) = rules {
                let rules_text = match fs::read_to_string(rules_path) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Error reading rules: {}: {}", rules_path.display(), e);
                        process::exit(1);
                    }
                };
                let custom = match custom_rules::parse_rules(&rules_text) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Error parsing rules: {}", e);
                        process::exit(1);
                    }
                };
                let mut custom_violations = custom_rules::evaluate_rules(&custom, &model);
                custom_violations.retain(|v| v.severity >= min_severity);
                violations.extend(custom_violations);
            }

            match format.as_str() {
                "json" => print_violations_json(&violations),
                "sarif" => print_violations_sarif(&violations, &source),
                _ => print_violations_text(&violations),
            }

            let has_errors = violations
                .iter()
                .any(|v| v.severity == check::Severity::Error);
            let has_warnings = violations
                .iter()
                .any(|v| v.severity == check::Severity::Warning);

            if has_errors {
                process::exit(2);
            } else if has_warnings {
                process::exit(1);
            }
        }
        Commands::Watch {
            source,
            out,
            style,
            baseline,
        } => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime");
            rt.block_on(serve::run_watch(source, out, style, baseline));
        }
        Commands::Serve {
            source,
            out,
            style,
            port,
            baseline,
            present,
        } => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime");
            if present {
                eprintln!("Presentation mode: http://localhost:{}/present.html", port);
            }
            rt.block_on(serve::run_serve(
                source, out, style, baseline, port, present,
            ));
        }
        Commands::Export {
            source,
            format,
            out,
        } => {
            let model = load_model(&source);
            let output = match format.as_str() {
                "json" => export::to_json(&model),
                "yaml" | "yml" => export::to_yaml(&model),
                _ => {
                    eprintln!("Error: --format must be 'json' or 'yaml'");
                    process::exit(1);
                }
            };
            if let Some(ref path) = out {
                fs::write(path, &output).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", path.display(), e);
                    process::exit(1);
                });
                eprintln!("Wrote: {}", path.display());
            } else {
                println!("{}", output);
            }
        }
        Commands::Import { source, out } => {
            let text = match fs::read_to_string(&source) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error: {}: {}", source.display(), e);
                    process::exit(1);
                }
            };
            let forge_text = match import::import_to_forge(&text) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            if let Some(ref path) = out {
                fs::write(path, &forge_text).unwrap_or_else(|e| {
                    eprintln!("Error writing {}: {}", path.display(), e);
                    process::exit(1);
                });
                eprintln!("Wrote: {}", path.display());
            } else {
                println!("{}", forge_text);
            }
        }
        Commands::Mcp { source } => {
            mcp::run(source);
        }
        Commands::Lsp => {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create tokio runtime")
                .block_on(lsp::run());
        }
    }
}

fn print_violations_text(violations: &[check::Violation]) {
    if violations.is_empty() {
        eprintln!("No issues found.");
        return;
    }
    eprintln!("Found {} issue(s):\n", violations.len());
    for v in violations {
        println!("{}", v);
    }
}

fn print_violations_json(violations: &[check::Violation]) {
    println!("[");
    for (i, v) in violations.iter().enumerate() {
        let comma = if i + 1 < violations.len() { "," } else { "" };
        let id = v
            .element_id
            .as_deref()
            .map(|s| format!("\"{}\"", s))
            .unwrap_or_else(|| "null".to_string());
        println!(
            "  {{\"rule\": \"{}\", \"severity\": \"{}\", \"element\": {}, \"message\": \"{}\"}}{}",
            v.rule,
            v.severity,
            id,
            v.message.replace('"', "\\\""),
            comma
        );
    }
    println!("]");
}

fn print_violations_sarif(violations: &[check::Violation], source: &Path) {
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
                        "artifactLocation": {
                            "uri": source.display().to_string(),
                        },
                        "region": { "startLine": 1 }
                    },
                    "logicalLocations": v.element_id.as_ref().map(|id| vec![json!({
                        "name": id,
                        "kind": "element"
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
                    "informationUri": "https://github.com/anthropics/forge",
                    "rules": violations.iter().map(|v| json!({
                        "id": v.rule,
                        "shortDescription": { "text": v.rule },
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

fn load_model(source: &Path) -> model::Model {
    let text = match fs::read_to_string(source) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error: {}: {}", source.display(), e);
            process::exit(1);
        }
    };
    let base_dir = source.parent().unwrap_or(Path::new("."));
    match parser::parse_with_preprocess(&text, base_dir) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
