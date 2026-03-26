//! Phase 1 scaffolding for rollback integration tests.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new("tests/fixtures/deterministic_reconciliation/rollback").to_path_buf()
}
