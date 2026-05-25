//! Subcommand handlers for the `forge` CLI. Each module owns one subcommand;
//! `main.rs` is reduced to clap definitions and a dispatch match.

mod analyze;
mod build;
mod check;
mod generate;
mod io;
mod serve;
mod util;

pub use analyze::cmd_analyze;
pub use build::cmd_build;
pub use check::cmd_check;
pub use generate::{cmd_generate, cmd_generate_catalog};
pub use io::{cmd_export, cmd_import};
pub use serve::{cmd_serve, cmd_watch};
