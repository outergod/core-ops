use crate::core::errors::CoreError;
use crate::core::release_governance::{
    parse_cargo_version, GovernanceRepositoryInput, ReleaseFragment, ReleaseFragmentFrontMatter,
    RepoChange, RepoChangeKind,
};
use crate::core::types::FailureClass;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn load_governance_repository_input(
    repo_root: &Path,
    base_ref: Option<&str>,
    head_ref: Option<&str>,
) -> Result<GovernanceRepositoryInput, CoreError> {
    let changed_files = load_repo_changes(repo_root, base_ref, head_ref)?;
    let changed_fragment_paths = changed_files
        .iter()
        .filter_map(|change| {
            // Destination path covers additions, modifications, and renames into changes/.
            if change.path.starts_with("changes/")
                && change.path.ends_with(".md")
                && change.path != "changes/README.md"
            {
                return Some(change.path.clone());
            }
            // For renames out of changes/, track the source path so that removing
            // a release-intent artifact is not silently treated as no fragment change.
            if change.kind == RepoChangeKind::Renamed {
                if let Some(prev) = &change.previous_path {
                    if prev.starts_with("changes/")
                        && prev.ends_with(".md")
                        && prev != "changes/README.md"
                    {
                        return Some(prev.clone());
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>();

    // When head_ref is provided the caller has specified an explicit commit to
    // validate.  Read the governance source files (fragments, Cargo.toml,
    // CHANGELOG.md) from that ref so that the check reflects the same revision
    // as the diff, not whatever happens to be in the working tree.
    let (fragments, cargo_version_after, changelog_contents) = if let Some(head_ref) = head_ref {
        let fragments = load_release_fragments_at_ref(repo_root, head_ref)?;
        let cargo_toml =
            run_git(repo_root, &["show", &format!("{head_ref}:Cargo.toml")]).map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!("failed to read Cargo.toml at {head_ref}: {err}"),
                )
            })?;
        let cargo_version_after = parse_cargo_version(&cargo_toml)?;
        let changelog_contents = run_git(repo_root, &["show", &format!("{head_ref}:CHANGELOG.md")])
            .map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!("failed to read CHANGELOG.md at {head_ref}: {err}"),
                )
            })?;
        (fragments, cargo_version_after, changelog_contents)
    } else {
        let fragments = load_release_fragments(repo_root)?;
        let cargo_version_after = parse_cargo_version(
            &fs::read_to_string(repo_root.join("Cargo.toml")).map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!("failed to read Cargo.toml: {err}"),
                )
            })?,
        )?;
        let changelog_contents =
            fs::read_to_string(repo_root.join("CHANGELOG.md")).map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!("failed to read CHANGELOG.md: {err}"),
                )
            })?;
        (fragments, cargo_version_after, changelog_contents)
    };

    let cargo_version_before = load_cargo_version_before(repo_root, base_ref, head_ref)?;

    Ok(GovernanceRepositoryInput {
        changed_files,
        changed_fragment_paths,
        fragments,
        cargo_version_before,
        cargo_version_after,
        changelog_contents,
    })
}

pub fn load_release_fragments(repo_root: &Path) -> Result<Vec<ReleaseFragment>, CoreError> {
    let changes_dir = repo_root.join("changes");
    if !changes_dir.exists() {
        return Ok(Vec::new());
    }
    let mut fragments = Vec::new();
    for entry in fs::read_dir(&changes_dir).map_err(|err| {
        CoreError::new(
            FailureClass::Validation,
            format!("failed to read changes directory: {err}"),
        )
    })? {
        let entry = entry.map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("failed to read changes entry: {err}"),
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
            continue;
        }
        fragments.push(load_release_fragment(repo_root, &path)?);
    }
    fragments.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(fragments)
}

fn load_release_fragments_at_ref(
    repo_root: &Path,
    git_ref: &str,
) -> Result<Vec<ReleaseFragment>, CoreError> {
    let tree = match run_git(repo_root, &["ls-tree", "--name-only", git_ref, "changes/"]) {
        Ok(output) => output,
        Err(_) => return Ok(Vec::new()),
    };
    let mut fragments = Vec::new();
    for path in tree.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if !path.ends_with(".md") || path == "changes/README.md" {
            continue;
        }
        let contents = run_git(repo_root, &["show", &format!("{git_ref}:{path}")])?;
        let (front_matter_str, body) = split_front_matter(&contents)?;
        let front_matter =
            serde_yaml::from_str::<ReleaseFragmentFrontMatter>(front_matter_str).map_err(
                |err| {
                    CoreError::new(
                        FailureClass::Validation,
                        format!("invalid release fragment {path}: {err}"),
                    )
                },
            )?;
        if front_matter.summary.trim().is_empty() {
            return Err(CoreError::new(
                FailureClass::Validation,
                format!(
                    "release fragment {path} has a blank summary; a non-empty summary is required"
                ),
            ));
        }
        fragments.push(ReleaseFragment {
            path: path.replace('\\', "/").to_string(),
            front_matter,
            body: body.trim().to_string(),
        });
    }
    fragments.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(fragments)
}

pub fn load_release_fragment(
    repo_root: &Path,
    path: &Path,
) -> Result<ReleaseFragment, CoreError> {
    let contents = fs::read_to_string(path).map_err(|err| {
        CoreError::new(
            FailureClass::Validation,
            format!("failed to read release fragment {}: {err}", path.display()),
        )
    })?;
    let (front_matter, body) = split_front_matter(&contents)?;
    let front_matter = serde_yaml::from_str::<ReleaseFragmentFrontMatter>(front_matter).map_err(
        |err| {
            CoreError::new(
                FailureClass::Validation,
                format!("invalid release fragment {}: {err}", path.display()),
            )
        },
    )?;
    if front_matter.summary.trim().is_empty() {
        return Err(CoreError::new(
            FailureClass::Validation,
            format!(
                "release fragment {} has a blank summary; a non-empty summary is required",
                path.display()
            ),
        ));
    }
    let relative = path
        .strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    Ok(ReleaseFragment {
        path: relative,
        front_matter,
        body: body.trim().to_string(),
    })
}

fn split_front_matter(contents: &str) -> Result<(&str, &str), CoreError> {
    let trimmed = contents.trim_start();
    let rest = trimmed.strip_prefix("---\n").ok_or_else(|| {
        CoreError::new(
            FailureClass::Validation,
            "release fragment must begin with YAML front matter",
        )
    })?;
    let end = rest.find("\n---").ok_or_else(|| {
        CoreError::new(
            FailureClass::Validation,
            "release fragment front matter is missing a closing delimiter",
        )
    })?;
    let front_matter = &rest[..end];
    let body = &rest[end + "\n---".len()..];
    Ok((front_matter, body))
}

pub fn load_repo_changes(
    repo_root: &Path,
    base_ref: Option<&str>,
    head_ref: Option<&str>,
) -> Result<Vec<RepoChange>, CoreError> {
    if head_ref.is_some() && base_ref.is_none() {
        return Err(CoreError::new(
            FailureClass::Validation,
            "--head-ref requires --base-ref; provide both to validate a specific commit range",
        ));
    }

    if let Some(base_ref) = base_ref {
        let mut changes = if let Some(head_ref) = head_ref {
            // Explicit ref-to-ref range: changed files come only from the diff
            // between the two commits; working-tree untracked files are excluded.
            parse_name_status_output(&run_git(
                repo_root,
                &[
                    "diff",
                    "--name-status",
                    "--find-renames",
                    &format!("{base_ref}..{head_ref}"),
                ],
            )?)?
        } else {
            // base_ref against working tree: include untracked files so that
            // new files not yet staged are still classified.
            let mut changes = parse_name_status_output(&run_git(
                repo_root,
                &["diff", "--name-status", "--find-renames", base_ref],
            )?)?;
            let untracked =
                run_git(repo_root, &["ls-files", "--others", "--exclude-standard"])?;
            for line in untracked.lines().filter(|line| !line.trim().is_empty()) {
                changes.push(RepoChange {
                    path: line.trim().to_string(),
                    previous_path: None,
                    kind: RepoChangeKind::Added,
                    before_contents: None,
                    after_contents: None,
                });
            }
            changes
        };
        enrich_repo_changes(repo_root, Some(base_ref), head_ref, &mut changes)?;
        return Ok(changes);
    }

    let working_tree = parse_name_status_output(&run_git(
        repo_root,
        &["diff", "--name-status", "--find-renames", "HEAD"],
    )?)?;
    let mut changes = working_tree;
    let untracked = run_git(repo_root, &["ls-files", "--others", "--exclude-standard"])?;
    for line in untracked.lines().filter(|line| !line.trim().is_empty()) {
        changes.push(RepoChange {
            path: line.trim().to_string(),
            previous_path: None,
            kind: RepoChangeKind::Added,
            before_contents: None,
            after_contents: None,
        });
    }
    if !changes.is_empty() {
        enrich_repo_changes(repo_root, Some("HEAD"), None, &mut changes)?;
        return Ok(changes);
    }

    let mut changes = parse_name_status_output(&run_git(
        repo_root,
        &["diff", "--name-status", "--find-renames", "HEAD~1..HEAD"],
    )?)?;
    enrich_repo_changes(repo_root, Some("HEAD~1"), Some("HEAD"), &mut changes)?;
    Ok(changes)
}

fn parse_name_status_output(output: &str) -> Result<Vec<RepoChange>, CoreError> {
    let mut changes = Vec::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let parts = line.split('\t').collect::<Vec<_>>();
        let status = parts[0];
        let change = if status.starts_with('R') {
            RepoChange {
                previous_path: parts.get(1).map(|value| (*value).to_string()),
                path: parts.get(2).unwrap_or(&"").to_string(),
                kind: RepoChangeKind::Renamed,
                before_contents: None,
                after_contents: None,
            }
        } else {
            RepoChange {
                previous_path: None,
                path: parts.get(1).unwrap_or(&"").to_string(),
                kind: match status.chars().next().unwrap_or('M') {
                    'A' => RepoChangeKind::Added,
                    'D' => RepoChangeKind::Deleted,
                    _ => RepoChangeKind::Modified,
                },
                before_contents: None,
                after_contents: None,
            }
        };
        changes.push(change);
    }
    Ok(changes)
}

fn enrich_repo_changes(
    repo_root: &Path,
    base_ref: Option<&str>,
    head_ref: Option<&str>,
    changes: &mut [RepoChange],
) -> Result<(), CoreError> {
    for change in changes {
        change.before_contents = load_change_contents(
            repo_root,
            base_ref,
            change.previous_path.as_deref().unwrap_or(&change.path),
        )?;
        change.after_contents = match change.kind {
            RepoChangeKind::Deleted => None,
            _ => load_change_contents(repo_root, head_ref, &change.path)?,
        };
    }
    Ok(())
}

fn load_change_contents(
    repo_root: &Path,
    git_ref: Option<&str>,
    relative_path: &str,
) -> Result<Option<String>, CoreError> {
    if let Some(git_ref) = git_ref {
        let object_ref = format!("{git_ref}:{relative_path}");
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .arg("show")
            .arg(&object_ref)
            .output()
            .map_err(|err| {
                CoreError::new(
                    FailureClass::Validation,
                    format!("failed to launch git: {err}"),
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("exists on disk, but not in")
                || stderr.contains("does not exist in")
                || stderr.contains("pathspec")
                || stderr.contains("unknown revision")
                || stderr.contains("fatal: invalid object name")
            {
                return Ok(None);
            }
            return Err(CoreError::new(
                FailureClass::Validation,
                format!("git show {object_ref} failed: {}", stderr.trim()),
            ));
        }

        return Ok(String::from_utf8(output.stdout).ok());
    }

    let path = repo_root.join(relative_path);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path).map_err(|err| {
        CoreError::new(
            FailureClass::Validation,
            format!("failed to read {}: {err}", path.display()),
        )
    })?;
    Ok(String::from_utf8(bytes).ok())
}

fn load_cargo_version_before(
    repo_root: &Path,
    base_ref: Option<&str>,
    head_ref: Option<&str>,
) -> Result<Option<String>, CoreError> {
    let reference = if let Some(base_ref) = base_ref {
        Some(base_ref.to_string())
    } else if !run_git(repo_root, &["diff", "--name-only", "HEAD"])?.trim().is_empty()
        || !run_git(repo_root, &["ls-files", "--others", "--exclude-standard"])?
            .trim()
            .is_empty()
    {
        Some("HEAD".to_string())
    } else if head_ref.unwrap_or("HEAD") == "HEAD" {
        Some("HEAD~1".to_string())
    } else {
        None
    };

    reference
        .map(|reference| {
            let contents = run_git(repo_root, &["show", &format!("{reference}:Cargo.toml")])?;
            parse_cargo_version(&contents)
        })
        .transpose()
}

fn run_git(repo_root: &Path, args: &[&str]) -> Result<String, CoreError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(args)
        .output()
        .map_err(|err| {
            CoreError::new(
                FailureClass::Validation,
                format!("failed to launch git: {err}"),
            )
        })?;

    if !output.status.success() {
        return Err(CoreError::new(
            FailureClass::Validation,
            format!(
                "git {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
