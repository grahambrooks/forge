mod analyze;
mod check;
mod generate;
mod layout;
mod model;
mod parser;
mod render;

use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
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
    },
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

            let text = match fs::read_to_string(&source) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error: {}: {}", source.display(), e);
                    process::exit(1);
                }
            };

            let model = match parser::parse(&text) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };

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
                let svg = render::render_svg(&lo, &style);

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
        } => {
            let text = match fs::read_to_string(&source) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error: {}: {}", source.display(), e);
                    process::exit(1);
                }
            };

            let model = match parser::parse(&text) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
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

            match generate::generate(&model, &config) {
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
        } => {
            let min_severity = match check::Severity::from_str(&severity) {
                Some(s) => s,
                None => {
                    eprintln!("Error: --severity must be 'error', 'warning', or 'info'");
                    process::exit(1);
                }
            };

            let text = match fs::read_to_string(&source) {
                Ok(t) => t,
                Err(e) => {
                    eprintln!("Error: {}: {}", source.display(), e);
                    process::exit(1);
                }
            };

            let model = match parser::parse(&text) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };

            let violations = check::check(&model, min_severity);

            if format == "json" {
                print_violations_json(&violations);
            } else {
                print_violations_text(&violations);
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
