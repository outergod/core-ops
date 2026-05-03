use crate::core::errors::CoreError;
use crate::core::types::FailureClass;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

const CHANGELOG_START_MARKER: &str = "<!-- core-ops-release:start -->";
const CHANGELOG_END_MARKER: &str = "<!-- core-ops-release:end -->";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseIntent {
    Patch,
    Minor,
    Major,
}

impl ReleaseIntent {
    pub fn label(self) -> &'static str {
        match self {
            ReleaseIntent::Patch => "patch",
            ReleaseIntent::Minor => "minor",
            ReleaseIntent::Major => "major",
        }
    }
}

impl PartialOrd for ReleaseIntent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ReleaseIntent {
    fn cmp(&self, other: &Self) -> Ordering {
        rank_for_intent(*self).cmp(&rank_for_intent(*other))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseClassification {
    Exempt,
    Releasable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepoChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoChange {
    pub path: String,
    pub previous_path: Option<String>,
    pub kind: RepoChangeKind,
    pub before_contents: Option<String>,
    pub after_contents: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseFragmentFrontMatter {
    pub change_id: String,
    pub release_intent: ReleaseIntent,
    pub summary: String,
    pub scope: Option<String>,
    #[serde(default)]
    pub release_preparation: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseFragment {
    pub path: String,
    pub front_matter: ReleaseFragmentFrontMatter,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GovernanceRepositoryInput {
    pub changed_files: Vec<RepoChange>,
    pub changed_fragment_paths: Vec<String>,
    pub fragments: Vec<ReleaseFragment>,
    pub cargo_version_before: Option<String>,
    pub cargo_version_after: String,
    pub changelog_contents: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GovernanceStatus {
    Passed,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GovernanceEvaluationResult {
    pub overall_status: GovernanceStatus,
    pub effective_classification: ReleaseClassification,
    pub effective_bump: Option<ReleaseIntent>,
    pub declared_bump: Option<ReleaseIntent>,
    pub version_bump: Option<ReleaseIntent>,
    pub missing_artifacts: Vec<String>,
    pub mismatch_reasons: Vec<String>,
    pub applied_rules: Vec<String>,
    pub changed_paths: Vec<String>,
    pub changed_fragment_paths: Vec<String>,
    pub release_preparation: bool,
    pub metadata_only: bool,
    pub changelog_aligned: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChangeAssessment {
    classification: ReleaseClassification,
    minimum_bump: Option<ReleaseIntent>,
    rule_id: &'static str,
}

pub fn evaluate_release_governance(
    input: &GovernanceRepositoryInput,
) -> Result<GovernanceEvaluationResult, CoreError> {
    let changed_paths = input
        .changed_files
        .iter()
        .map(|change| change.path.clone())
        .collect::<Vec<_>>();
    let metadata_only = !input.changed_files.is_empty()
        && input
            .changed_files
            .iter()
            .all(|change| is_metadata_path(&change.path));

    let fragment = resolve_changed_fragment(&input.changed_fragment_paths, &input.fragments)?;
    let release_preparation = fragment
        .as_ref()
        .map(|fragment| fragment.front_matter.release_preparation)
        .unwrap_or(false);
    let declared_bump = fragment
        .as_ref()
        .map(|fragment| fragment.front_matter.release_intent);
    let version_bump = input
        .cargo_version_before
        .as_deref()
        .map(|before| classify_version_bump(before, &input.cargo_version_after))
        .transpose()?
        .flatten();

    let mut applied_rules = Vec::new();
    let mut effective_bump: Option<ReleaseIntent> = None;
    let mut effective_classification = ReleaseClassification::Exempt;

    for change in input
        .changed_files
        .iter()
        .filter(|change| !is_metadata_path(&change.path))
    {
        if let Some(assessment) = assess_change(change) {
            applied_rules.push(assessment.rule_id.to_string());
            if assessment.classification == ReleaseClassification::Releasable {
                effective_classification = ReleaseClassification::Releasable;
            }
            if let Some(bump) = assessment.minimum_bump {
                effective_bump = Some(match effective_bump {
                    Some(current) => current.max(bump),
                    None => bump,
                });
            }
        }
    }

    if metadata_only && !input.changed_files.is_empty() {
        applied_rules.push("metadata_only_change_set".to_string());
    }

    let mut missing_artifacts = Vec::new();
    let mut mismatch_reasons = Vec::new();

    if effective_classification == ReleaseClassification::Releasable {
        for path in ["Cargo.toml", "CHANGELOG.md"] {
            if !changed_paths.iter().any(|changed| changed == path) {
                missing_artifacts.push(path.to_string());
            }
        }
        if input.changed_fragment_paths.is_empty() {
            missing_artifacts.push("changes/<change-id>.md".to_string());
        }
    }

    if input.changed_fragment_paths.len() > 1 {
        mismatch_reasons.push(
            "exactly one changed release fragment is allowed per change set".to_string(),
        );
    }

    // A listed fragment path that cannot be resolved (e.g. renamed out of changes/)
    // is treated as a missing release-intent artifact rather than a hard error.
    if !input.changed_fragment_paths.is_empty()
        && input.changed_fragment_paths.len() == 1
        && fragment.is_none()
    {
        missing_artifacts.push(input.changed_fragment_paths[0].clone());
    }

    if effective_classification == ReleaseClassification::Releasable && declared_bump.is_none() {
        missing_artifacts.push("release_intent".to_string());
    }

    if metadata_only && !release_preparation {
        mismatch_reasons.push(
            "metadata-only changes require release_preparation: true in the checked-in fragment"
                .to_string(),
        );
    }

    if effective_classification == ReleaseClassification::Releasable {
        match (declared_bump, effective_bump) {
            (Some(declared), Some(required)) if declared < required => {
                mismatch_reasons.push(format!(
                    "declared release intent {} is lower than required {}",
                    declared.label(),
                    required.label()
                ))
            }
            (None, Some(required)) => mismatch_reasons.push(format!(
                "required release intent {} is missing from the changed fragment",
                required.label()
            )),
            _ => {}
        }

        match (version_bump, effective_bump) {
            (Some(actual), Some(required)) if actual < required => mismatch_reasons.push(format!(
                "Cargo.toml version bump {} is lower than required {}",
                actual.label(),
                required.label()
            )),
            (None, Some(required)) => mismatch_reasons.push(format!(
                "Cargo.toml must change with a {} version bump",
                required.label()
            )),
            _ => {}
        }
    }

    let rendered_changelog =
        render_generated_changelog(&input.changelog_contents, &input.fragments)?;
    let changelog_aligned = rendered_changelog == input.changelog_contents;
    if effective_classification == ReleaseClassification::Releasable && !changelog_aligned {
        mismatch_reasons.push(
            "CHANGELOG.md does not match generated content from approved fragments".to_string(),
        );
    }

    let overall_status = if missing_artifacts.is_empty() && mismatch_reasons.is_empty() {
        GovernanceStatus::Passed
    } else {
        GovernanceStatus::Failed
    };

    Ok(GovernanceEvaluationResult {
        overall_status,
        effective_classification,
        effective_bump,
        declared_bump,
        version_bump,
        missing_artifacts,
        mismatch_reasons,
        applied_rules,
        changed_paths,
        changed_fragment_paths: input.changed_fragment_paths.clone(),
        release_preparation,
        metadata_only,
        changelog_aligned,
    })
}

pub fn classify_version_bump(
    before: &str,
    after: &str,
) -> Result<Option<ReleaseIntent>, CoreError> {
    let before_triplet = parse_semver_triplet(before)?;
    let after_triplet = parse_semver_triplet(after)?;

    if before_triplet == after_triplet {
        return Ok(None);
    }

    if after_triplet.0 > before_triplet.0 && after_triplet.1 == 0 && after_triplet.2 == 0 {
        return Ok(Some(ReleaseIntent::Major));
    }
    if after_triplet.0 == before_triplet.0
        && after_triplet.1 > before_triplet.1
        && after_triplet.2 == 0
    {
        return Ok(Some(ReleaseIntent::Minor));
    }
    if after_triplet.0 == before_triplet.0
        && after_triplet.1 == before_triplet.1
        && after_triplet.2 > before_triplet.2
    {
        return Ok(Some(ReleaseIntent::Patch));
    }

    Err(CoreError::new(
        FailureClass::Validation,
        format!("unsupported semantic version transition: {before} -> {after}"),
    ))
}

pub fn parse_cargo_version(contents: &str) -> Result<String, CoreError> {
    let manifest: toml::Value = toml::from_str(contents).map_err(|err| {
        CoreError::new(
            FailureClass::Validation,
            format!("failed to parse Cargo.toml: {err}"),
        )
    })?;
    let value = manifest
        .get("package")
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            CoreError::new(
                FailureClass::Validation,
                "unable to locate package version in Cargo.toml",
            )
        })?
        .to_string();
    parse_semver_triplet(&value)?;
    Ok(value)
}

/// Promote the rendered `[Unreleased]` block to a new
/// `## [<version>] - <date>` section. Idempotent: if the version
/// section already exists, returns `existing_contents` verbatim.
///
/// Pairs with `render_generated_changelog`: the rendered
/// `[Unreleased]` body (everything between the markers) becomes the
/// versioned section's body, then `[Unreleased]` is reset to bare
/// markers. Caller is responsible for deleting the consumed
/// fragments under `changes/`.
pub fn promote_changelog(
    existing_contents: &str,
    version: &str,
    date: &str,
) -> Result<String, CoreError> {
    let versioned_heading = format!("## [{version}]");
    if existing_contents
        .lines()
        .any(|line| line.starts_with(&versioned_heading))
    {
        // Already promoted — no-op so the pipeline can re-run safely.
        return Ok(existing_contents.to_string());
    }

    let unreleased_start = existing_contents.find("## [Unreleased]").ok_or_else(|| {
        CoreError::new(
            FailureClass::Validation,
            "CHANGELOG.md is missing the [Unreleased] section",
        )
    })?;
    let after_unreleased = &existing_contents[unreleased_start + "## [Unreleased]".len()..];
    let next_section_offset = after_unreleased
        .find("\n## [")
        .unwrap_or(after_unreleased.len());
    let next_section_abs = unreleased_start + "## [Unreleased]".len() + next_section_offset;

    let unreleased_block = &existing_contents[unreleased_start..next_section_abs];
    let start_idx = unreleased_block.find(CHANGELOG_START_MARKER).ok_or_else(|| {
        CoreError::new(
            FailureClass::Validation,
            "CHANGELOG.md [Unreleased] block is missing the start marker",
        )
    })?;
    let end_idx = unreleased_block.find(CHANGELOG_END_MARKER).ok_or_else(|| {
        CoreError::new(
            FailureClass::Validation,
            "CHANGELOG.md [Unreleased] block is missing the end marker",
        )
    })?;
    let unreleased_body = unreleased_block
        [start_idx + CHANGELOG_START_MARKER.len()..end_idx]
        .trim_matches('\n')
        .to_string();

    let prefix = &existing_contents[..unreleased_start];
    let suffix = &existing_contents[next_section_abs..];

    let mut output = String::new();
    output.push_str(prefix);
    output.push_str("## [Unreleased]\n\n");
    output.push_str(CHANGELOG_START_MARKER);
    output.push('\n');
    output.push_str(CHANGELOG_END_MARKER);
    output.push_str("\n\n");
    output.push_str(&format!("## [{version}] - {date}\n"));
    if !unreleased_body.is_empty() {
        output.push('\n');
        output.push_str(unreleased_body.trim_end_matches('\n'));
        output.push('\n');
    }
    // Suffix starts at the `\n## [...` of the next section (or EOF).
    // Strip its leading newlines so we can normalise to exactly one
    // blank line between sections — the Keep a Changelog convention.
    let suffix = suffix.trim_start_matches('\n');
    if !suffix.is_empty() {
        output.push('\n');
        output.push_str(suffix);
    }
    Ok(output)
}

pub fn render_generated_changelog(
    existing_contents: &str,
    fragments: &[ReleaseFragment],
) -> Result<String, CoreError> {
    let unreleased_start = existing_contents.find("## [Unreleased]").ok_or_else(|| {
        CoreError::new(
            FailureClass::Validation,
            "CHANGELOG.md is missing the [Unreleased] section",
        )
    })?;

    let after_unreleased = &existing_contents[unreleased_start + "## [Unreleased]".len()..];
    let next_section_offset = after_unreleased
        .find("\n## [")
        .unwrap_or(after_unreleased.len());
    let next_section = unreleased_start + "## [Unreleased]".len() + next_section_offset;

    let prefix = &existing_contents[..unreleased_start];
    let suffix = &existing_contents[next_section..];

    let mut generated = String::new();
    generated.push_str("## [Unreleased]\n\n");
    generated.push_str(CHANGELOG_START_MARKER);
    generated.push('\n');
    let pending_fragments: Vec<_> = fragments
        .iter()
        .filter(|f| !f.front_matter.release_preparation)
        .collect();
    if !pending_fragments.is_empty() {
        generated.push_str("### Changed\n\n");
        let mut summaries = pending_fragments
            .iter()
            .map(|fragment| fragment.front_matter.summary.trim().to_string())
            .collect::<Vec<_>>();
        summaries.sort();
        summaries.dedup();
        for summary in summaries {
            generated.push_str("- ");
            generated.push_str(&summary);
            generated.push('\n');
        }
    }
    generated.push_str(CHANGELOG_END_MARKER);
    generated.push('\n');

    Ok(format!("{prefix}{generated}{suffix}"))
}

fn resolve_changed_fragment<'a>(
    changed_fragment_paths: &[String],
    fragments: &'a [ReleaseFragment],
) -> Result<Option<&'a ReleaseFragment>, CoreError> {
    if changed_fragment_paths.is_empty() {
        return Ok(None);
    }
    if changed_fragment_paths.len() > 1 {
        return Ok(None);
    }
    let path = &changed_fragment_paths[0];
    Ok(fragments.iter().find(|fragment| &fragment.path == path))
}

fn assess_change(change: &RepoChange) -> Option<ChangeAssessment> {
    // For renames, assess source path as a deletion and destination as an
    // addition, then return whichever is more impactful. This prevents an
    // exempt destination from masking a releasable source removal.
    if change.kind == RepoChangeKind::Renamed {
        if let Some(prev) = change.previous_path.as_deref() {
            let source = assess_path(prev, RepoChangeKind::Deleted, None);
            let dest = assess_path(&change.path, RepoChangeKind::Added, None);
            return take_higher_impact(source, dest);
        }
    }
    assess_path(&change.path, change.kind, Some(change))
}

fn take_higher_impact(
    a: Option<ChangeAssessment>,
    b: Option<ChangeAssessment>,
) -> Option<ChangeAssessment> {
    match (a, b) {
        (None, b) => b,
        (a, None) => a,
        (Some(a), Some(b)) => Some(if assessment_dominates(&a, &b) { a } else { b }),
    }
}

fn assessment_dominates(a: &ChangeAssessment, b: &ChangeAssessment) -> bool {
    if a.classification != b.classification {
        return a.classification == ReleaseClassification::Releasable;
    }
    let rank = |bump: Option<ReleaseIntent>| bump.map(rank_for_intent).unwrap_or(0);
    rank(a.minimum_bump) >= rank(b.minimum_bump)
}

fn assess_path(
    path: &str,
    kind: RepoChangeKind,
    change: Option<&RepoChange>,
) -> Option<ChangeAssessment> {
    if is_always_exempt_path(path) {
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Exempt,
            minimum_bump: None,
            rule_id: "always_exempt_documentation_or_formatting",
        });
    }

    if path.starts_with("tests/fixtures/verification/scenarios/") {
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Releasable,
            minimum_bump: Some(ReleaseIntent::Patch),
            rule_id: "accepted_verification_corpus_patch_floor",
        });
    }

    if path.starts_with("tests/fixtures/provenance_state/") && path.ends_with(".json") {
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Releasable,
            minimum_bump: Some(ReleaseIntent::Patch),
            rule_id: "contract_fixture_provenance_state",
        });
    }

    if path.starts_with("tests/fixtures/verification/artifacts/") && path.ends_with(".json") {
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Releasable,
            minimum_bump: Some(ReleaseIntent::Patch),
            rule_id: "contract_fixture_verification_artifact",
        });
    }

    if path.starts_with("src/") {
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Releasable,
            minimum_bump: match kind {
                RepoChangeKind::Added => Some(ReleaseIntent::Minor),
                RepoChangeKind::Deleted | RepoChangeKind::Renamed => Some(ReleaseIntent::Major),
                RepoChangeKind::Modified => Some(ReleaseIntent::Patch),
            },
            rule_id: "rust_source_surface",
        });
    }

    if path.starts_with("tests/fixtures/distribution/") && path.ends_with(".json") {
        if path == "tests/fixtures/distribution/release-metadata.json"
            && change.is_some_and(is_release_metadata_version_sync)
        {
            return Some(ChangeAssessment {
                classification: ReleaseClassification::Exempt,
                minimum_bump: None,
                rule_id: "release_metadata_version_sync",
            });
        }
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Releasable,
            minimum_bump: Some(ReleaseIntent::Major),
            rule_id: "machine_readable_distribution_contract",
        });
    }

    if path.starts_with("tests/integration/test_distribution_")
        || path.starts_with("tests/integration/test_verification_")
    {
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Releasable,
            minimum_bump: Some(ReleaseIntent::Patch),
            rule_id: "release_or_verification_claim_test_surface",
        });
    }

    if path == "README.md" {
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Releasable,
            minimum_bump: Some(match kind {
                RepoChangeKind::Deleted | RepoChangeKind::Renamed => ReleaseIntent::Major,
                RepoChangeKind::Added => ReleaseIntent::Minor,
                RepoChangeKind::Modified => ReleaseIntent::Patch,
            }),
            rule_id: "packaged_readme_surface",
        });
    }

    if path.starts_with("tests/")
        || path.starts_with("specs/")
        || path.starts_with("docs/")
        || path.starts_with(".github/")
        || path == "AGENTS.md"
    {
        return Some(ChangeAssessment {
            classification: ReleaseClassification::Exempt,
            minimum_bump: None,
            rule_id: "context_dependent_non_public_docs_or_tests",
        });
    }

    Some(ChangeAssessment {
        classification: ReleaseClassification::Releasable,
        minimum_bump: Some(match kind {
            RepoChangeKind::Added => ReleaseIntent::Minor,
            RepoChangeKind::Deleted | RepoChangeKind::Renamed => ReleaseIntent::Major,
            RepoChangeKind::Modified => ReleaseIntent::Patch,
        }),
        rule_id: "unclassified_path_releasable_default",
    })
}

fn is_always_exempt_path(path: &str) -> bool {
    path.starts_with("docs/")
        || path.starts_with("specs/")
        || (path.ends_with(".md")
            && path != "README.md"
            && path != "CHANGELOG.md"
            && !path.starts_with("changes/"))
}

pub fn is_metadata_path(path: &str) -> bool {
    path == "Cargo.toml"
        || path == "CHANGELOG.md"
        || path.starts_with("changes/") && path != "changes/README.md"
}

fn is_release_metadata_version_sync(change: &RepoChange) -> bool {
    let Some(before) = change.before_contents.as_deref() else {
        return false;
    };
    let Some(after) = change.after_contents.as_deref() else {
        return false;
    };

    let Ok(before_json) = serde_json::from_str::<serde_json::Value>(before) else {
        return false;
    };
    let Ok(after_json) = serde_json::from_str::<serde_json::Value>(after) else {
        return false;
    };

    let (Some(before_object), Some(after_object)) =
        (before_json.as_object(), after_json.as_object())
    else {
        return false;
    };

    if before_object.len() != after_object.len()
        || before_object.keys().collect::<Vec<_>>() != after_object.keys().collect::<Vec<_>>()
    {
        return false;
    }

    let changed_keys = before_object
        .iter()
        .filter_map(|(key, before_value)| {
            after_object
                .get(key)
                .filter(|after_value| *after_value != before_value)
                .map(|_| key.as_str())
        })
        .collect::<Vec<_>>();

    changed_keys == ["latest_release_identity"]
}

fn parse_semver_triplet(version: &str) -> Result<(u64, u64, u64), CoreError> {
    let base = version.split('-').next().unwrap_or(version);
    let parts = base.split('.').collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(CoreError::new(
            FailureClass::Validation,
            format!("invalid semantic version: {version}"),
        ));
    }
    let major = parts[0].parse::<u64>().map_err(|_| {
        CoreError::new(
            FailureClass::Validation,
            format!("invalid semantic version: {version}"),
        )
    })?;
    let minor = parts[1].parse::<u64>().map_err(|_| {
        CoreError::new(
            FailureClass::Validation,
            format!("invalid semantic version: {version}"),
        )
    })?;
    let patch = parts[2].parse::<u64>().map_err(|_| {
        CoreError::new(
            FailureClass::Validation,
            format!("invalid semantic version: {version}"),
        )
    })?;
    Ok((major, minor, patch))
}

fn rank_for_intent(intent: ReleaseIntent) -> u8 {
    match intent {
        ReleaseIntent::Patch => 1,
        ReleaseIntent::Minor => 2,
        ReleaseIntent::Major => 3,
    }
}
