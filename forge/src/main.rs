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
    }
}
