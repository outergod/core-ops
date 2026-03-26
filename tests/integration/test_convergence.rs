//! Phase 1 scaffolding for convergence and non-convergence integration tests.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new("tests/fixtures/deterministic_reconciliation").to_path_buf()
}
