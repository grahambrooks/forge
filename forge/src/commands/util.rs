//! Helpers shared by every subcommand handler.

use std::fs;
use std::path::Path;
use std::process;

use crate::{model, parser};

pub(crate) fn load_model(source: &Path) -> model::Model {
    let text =
        fs::read_to_string(source).unwrap_or_else(|e| die(&format!("{}: {}", source.display(), e)));
    let base_dir = source.parent().unwrap_or(Path::new("."));
    parser::parse_with_preprocess(&text, base_dir).unwrap_or_else(|e| die(&format!("{}", e)))
}

pub(crate) fn write_or_stdout(out: Option<&Path>, content: &str) {
    if let Some(path) = out {
        fs::write(path, content)
            .unwrap_or_else(|e| die(&format!("writing {}: {}", path.display(), e)));
        eprintln!("Wrote: {}", path.display());
    } else {
        println!("{}", content);
    }
}

pub(crate) fn die(msg: &str) -> ! {
    eprintln!("Error: {}", msg);
    process::exit(1);
}
