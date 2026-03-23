use core_ops::io::repo::load_desired_state;

#[test]
fn repo_unavailable_returns_error() {
    let result = load_desired_state("/does/not/exist", "main");
    assert!(result.is_err());
}
