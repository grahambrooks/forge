mod analyze;
mod animate;
mod catalog_parser;
mod check;
mod commands;
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
mod png;
mod preprocess;
mod render;
mod serve;
mod text;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use commands::{
    cmd_analyze, cmd_build, cmd_check, cmd_export, cmd_generate, cmd_generate_catalog, cmd_import,
    cmd_serve, cmd_watch,
};

// ─── CLI Definition ──────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "forge",
    version = env!("FORGE_VERSION"),
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
        #[arg(long, default_value = "outline")]
        style: String,

        /// Output format: svg, png, both
        #[arg(long, default_value = "svg")]
        format: String,

        /// PNG scale factor (1.0 = SVG-native pixels, 2.0 = retina)
        #[arg(long, default_value = "2.0")]
        scale: f32,
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

        /// Comma-separated scanner list (code,semantic,ci,docker,git,k8s,infra,diagrams)
        #[arg(long, default_value = "code,semantic,ci,docker,git,k8s,infra,diagrams")]
        scanners: String,

        /// Exclude directory names (repeatable)
        #[arg(long)]
        exclude: Vec<String>,

        /// Print output to stdout without writing file
        #[arg(long)]
        dry_run: bool,

        /// Merge fresh analysis into an existing .forge, preserving user
        /// content. Only elements tagged `inferred` are refreshed. Safe to
        /// re-run in CI over a hand-authored model.
        #[arg(long)]
        merge: Option<PathBuf>,

        /// External plugin command to run during analysis. Repeatable.
        /// Whitespace splits argv: `--plugin "tsx ./plugin.ts"`.
        #[arg(long)]
        plugin: Vec<String>,
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
        #[arg(long, default_value = "outline")]
        style: String,

        /// Baseline .forge file to diff against (highlights changes)
        #[arg(long)]
        baseline: Option<PathBuf>,
    },

    /// Generate a multi-project catalog documentation website
    GenerateCatalog {
        /// Input .forge-catalog file
        #[arg(short, long, default_value = "forge.catalog")]
        source: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "_site")]
        out: PathBuf,

        /// Site title (defaults to catalog name)
        #[arg(long)]
        title: Option<String>,

        /// Base URL for deployment (e.g. /docs/ for subdirectory)
        #[arg(long, default_value = "/")]
        base_url: String,

        /// Diagram rendering style: filled or outline
        #[arg(long, default_value = "outline")]
        style: String,

        /// Disable incremental builds (regenerate all projects)
        #[arg(long)]
        no_incremental: bool,
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
        #[arg(long, default_value = "outline")]
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
        #[arg(long, default_value = "outline")]
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
        /// Input .forge file. If omitted, the server starts with an empty
        /// model and expects the client to populate it via `forge_analyze`.
        #[arg(short, long)]
        source: Option<PathBuf>,
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
            format,
            scale,
        } => cmd_build(&source, view.as_deref(), &out, &style, &format, scale),
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
            merge,
            plugin,
        } => cmd_analyze(
            paths,
            &out,
            &scanners,
            exclude,
            dry_run,
            merge.as_deref(),
            plugin,
        ),
        Commands::Generate {
            source,
            out,
            title,
            base_url,
            style,
            baseline,
        } => cmd_generate(&source, &out, title, &base_url, &style, baseline.as_deref()),
        Commands::GenerateCatalog {
            source,
            out,
            title,
            base_url,
            style,
            no_incremental,
        } => cmd_generate_catalog(&source, &out, title, &base_url, &style, !no_incremental),
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
        Commands::Mcp { source } => {
            if let Some(path) = &source {
                if !path.exists() {
                    eprintln!(
                        "Warning: source {} does not exist; starting with empty model.",
                        path.display()
                    );
                    mcp::run(None);
                    return;
                }
            }
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

// All `cmd_*` handlers, their printers, and helpers live under `commands/`.
