use std::fs;
use std::path::Path;

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

#[test]
fn root_distribution_materials_exist_and_are_linked_from_readme() {
    let root = repo_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("read README");

    for file in ["LICENSE", "CODE_OF_CONDUCT.md", "CHANGELOG.md"] {
        assert!(root.join(file).exists(), "missing {file}");
        assert!(readme.contains(file), "README should reference {file}");
    }
}

#[test]
fn license_and_code_of_conduct_are_discoverable() {
    let root = repo_root();
    let license = fs::read_to_string(root.join("LICENSE")).expect("read LICENSE");
    let coc = fs::read_to_string(root.join("CODE_OF_CONDUCT.md")).expect("read CoC");

    assert!(license.contains("GNU Affero General Public License"));
    assert!(license.contains("AGPLv3+"));
    assert!(coc.contains("# Code of Conduct"));
    assert!(coc.contains("Be constructive."));
}
