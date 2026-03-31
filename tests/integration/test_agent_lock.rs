use std::path::PathBuf;

use core_ops::core::errors::RunLockError;
use core_ops::core::types::RunLock;
use core_ops::io::lock::FileRunLock;

fn temp_dir(prefix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!("{}_{}", prefix, nanos));
    path
}

#[test]
fn run_lock_prevents_overlap() {
    let dir = temp_dir("core_ops_lock");
    let path = dir.join("agent.lock");
    std::fs::create_dir_all(&dir).expect("create temp dir");

    let lock = FileRunLock::new(&path);
    let guard = lock.acquire().expect("acquire lock");
    let second = lock.acquire();

    assert!(matches!(second, Err(RunLockError::AlreadyHeld)));

    lock.release(guard).expect("release lock");
}
