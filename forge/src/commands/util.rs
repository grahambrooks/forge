//! Helpers shared by every subcommand handler.

use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process;

use crate::{model, parser};

/// Loads and parses a model. A `source` of `-` reads the model from stdin and
/// resolves `!include` paths against the working directory — this is how
/// editors render a buffer that has not been saved yet.
pub(crate) fn load_model(source: &Path) -> model::Model {
    let (text, base_dir) = if source == Path::new("-") {
        let mut text = String::new();
        io::stdin()
            .read_to_string(&mut text)
            .unwrap_or_else(|e| die(&format!("reading stdin: {}", e)));
        (text, Path::new("."))
    } else {
        let text = fs::read_to_string(source)
            .unwrap_or_else(|e| die(&format!("{}: {}", source.display(), e)));
        (text, source.parent().unwrap_or(Path::new(".")))
    };
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
