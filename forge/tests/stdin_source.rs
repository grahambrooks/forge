//! `--source -` reads the model from stdin, so editors can render unsaved buffers.

use std::io::Write;
use std::process::{Command, Stdio};

fn forge_with_stdin(args: &[&str], cwd: &std::path::Path, stdin: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn forge");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn build_renders_model_read_from_stdin() {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/examples/payments.forge"
    ))
    .unwrap();
    let out = tempfile::tempdir().unwrap();
    let out_arg = out.path().to_str().unwrap();

    let result = forge_with_stdin(
        &["build", "--source", "-", "--out", out_arg],
        out.path(),
        &source,
    );

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(out.path().join("SystemContext.svg").exists());
}

#[test]
fn stdin_includes_resolve_from_working_directory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("people.forge"),
        "model {\n  customer = person \"Customer\"\n}\n",
    )
    .unwrap();
    let model = "forge \"Inc\" {\n  !include people.forge\n}\n";

    let result = forge_with_stdin(
        &["export", "--source", "-", "--format", "json"],
        dir.path(),
        model,
    );

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("\"customer\""));
}
