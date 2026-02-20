use qai_cli::{copy, info, read_prompt, tools, validate};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

fn temp_prompt(content: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prompt.md");
    fs::write(&path, content).unwrap();
    (dir, path)
}

// ── read_prompt ───────────────────────────────────────────────────────────────

#[test]
fn read_prompt_returns_content() {
    let (_dir, path) = temp_prompt("hello world");
    assert_eq!(read_prompt(&path).unwrap(), "hello world");
}

#[test]
fn read_prompt_missing_file_errors() {
    let path = PathBuf::from("/nonexistent/path/prompt.md");
    assert!(read_prompt(&path).is_err());
}

// ── validate ──────────────────────────────────────────────────────────────────

#[test]
fn validate_passes_with_all_sections() {
    let content = "## ENVIRONMENT\n### PRIMARY OBJECTIVE\n### MODE SELECTION PRIMER\n";
    let (_dir, path) = temp_prompt(content);
    assert!(validate(&path).is_ok());
}

#[test]
fn validate_fails_when_section_missing() {
    let content = "## ENVIRONMENT\n### PRIMARY OBJECTIVE\n";
    let (_dir, path) = temp_prompt(content);
    let err = validate(&path).unwrap_err();
    assert!(err.to_string().contains("MODE SELECTION PRIMER"));
}

#[test]
fn validate_fails_for_empty_file() {
    let (_dir, path) = temp_prompt("");
    let err = validate(&path).unwrap_err();
    assert!(err.to_string().contains("Missing sections"));
}

// ── copy ──────────────────────────────────────────────────────────────────────

#[test]
fn copy_writes_content_to_dest() {
    let (_src_dir, src) = temp_prompt("system prompt content");
    let dest_dir = TempDir::new().unwrap();
    let dest = dest_dir.path().join("out.md");
    copy(&src, dest.clone(), false).unwrap();
    assert_eq!(fs::read_to_string(&dest).unwrap(), "system prompt content");
}

#[test]
fn copy_fails_if_dest_exists_without_force() {
    let (_src_dir, src) = temp_prompt("content");
    let dest_dir = TempDir::new().unwrap();
    let dest = dest_dir.path().join("out.md");
    fs::write(&dest, "old").unwrap();
    let err = copy(&src, dest, false).unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn copy_overwrites_with_force() {
    let (_src_dir, src) = temp_prompt("new content");
    let dest_dir = TempDir::new().unwrap();
    let dest = dest_dir.path().join("out.md");
    fs::write(&dest, "old").unwrap();
    copy(&src, dest.clone(), true).unwrap();
    assert_eq!(fs::read_to_string(&dest).unwrap(), "new content");
}

// ── info ──────────────────────────────────────────────────────────────────────

#[test]
fn info_runs_without_error_for_existing_file() {
    let (_dir, path) = temp_prompt("content");
    assert!(info(&path).is_ok());
}

#[test]
fn info_runs_without_error_for_missing_file() {
    let path = PathBuf::from("/nonexistent/prompt.md");
    assert!(info(&path).is_ok());
}

// ── show ──────────────────────────────────────────────────────────────────────

#[test]
fn show_prints_prompt_content() {
    let (_dir, path) = temp_prompt("hello from show");
    assert!(qai_cli::show(&path).is_ok());
}

#[test]
fn show_errors_for_missing_file() {
    let path = PathBuf::from("/nonexistent/show_prompt.md");
    assert!(qai_cli::show(&path).is_err());
}

// ── tools ─────────────────────────────────────────────────────────────────────

#[test]
fn tools_runs_without_error() {
    assert!(tools().is_ok());
}
