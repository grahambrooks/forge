mod model;
mod parser;
mod layout;
mod render;

use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Usage: forge build [--source FILE] [--view NAME] [--out DIR]
    let mut source = PathBuf::from("forge.forge");
    let mut view_filter: Option<String> = None;
    let mut out_dir = PathBuf::from("forge-output");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "build" => {} // command, just skip
            "--source" => {
                i += 1;
                if i < args.len() { source = PathBuf::from(&args[i]); }
            }
            "--view" => {
                i += 1;
                if i < args.len() { view_filter = Some(args[i].clone()); }
            }
            "--out" => {
                i += 1;
                if i < args.len() { out_dir = PathBuf::from(&args[i]); }
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            _ => {
                // If it looks like a file path without --source flag
                if args[i].ends_with(".forge") {
                    source = PathBuf::from(&args[i]);
                }
            }
        }
        i += 1;
    }

    // Read input
    let input = match fs::read_to_string(&source) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", source.display(), e);
            std::process::exit(1);
        }
    };

    // Parse
    let model = match parser::parse(&input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    eprintln!("Parsed model: \"{}\"", model.name);
    eprintln!("  Elements: {}", model.elements.len());
    eprintln!("  Relationships: {}", model.relationships.len());
    eprintln!("  Views: {}", model.views.len());

    // Create output directory
    fs::create_dir_all(&out_dir).ok();

    // Render each view
    for view in &model.views {
        if let Some(ref filter) = view_filter {
            if &view.key != filter {
                continue;
            }
        }

        let view_layout = match view.kind {
            model::ViewKind::SystemContext => layout::layout_system_context_view(&model, view),
            model::ViewKind::Container => layout::layout_container_view(&model, view),
            model::ViewKind::PipelineView => layout::layout_pipeline_view(&model, view),
        };

        let svg = render::render_svg(&view_layout);

        let filename = format!("{}.svg", view.key);
        let path = out_dir.join(&filename);
        match fs::write(&path, &svg) {
            Ok(()) => eprintln!("  Wrote: {}", path.display()),
            Err(e) => eprintln!("  Error writing {}: {}", path.display(), e),
        }
    }

    eprintln!("Done.");
}

fn print_help() {
    eprintln!(
        r#"forge — A unified software modeling tool (prototype)

USAGE:
    forge build [OPTIONS]

OPTIONS:
    --source <FILE>    Input .forge file (default: ./forge.forge)
    --view <NAME>      Render a specific view (default: all)
    --out <DIR>        Output directory (default: ./forge-output/)
    -h, --help         Show this help

EXAMPLES:
    forge build --source examples/payments.forge
    forge build --source examples/payments.forge --view Containers
    forge build --source examples/payments.forge --out ./docs/diagrams/
"#
    );
}
