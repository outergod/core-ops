# Data Model: SemVer and Changelog Governance

## Release Fragment

- **Purpose**: Captures per-change SemVer intent and release-note content for a
  releasable pull request.
- **Location**: `changes/<change-id>.md`
- **Core fields**:
  - `change_id`: stable identifier for the fragment within the branch
  - `release_intent`: one of `patch`, `minor`, `major`
  - `summary`: short human-readable release note
  - `scope`: optional short label describing the affected contract area
  - `release_preparation`: boolean indicating whether the change is
    intentionally metadata-only release preparation
- **Validation rules**:
  - `release_intent` must be present and valid
  - `summary` must be non-empty and human-readable
  - `release_preparation` defaults to `false`
  - exactly one effective fragment set must be attributable to the pull request

## Release Classification Rule

- **Purpose**: Determines whether changed content is exempt, releasable, or
  context-dependent.
- **Core fields**:
  - `rule_id`
  - `classification`: `exempt`, `releasable`, or `context-dependent`
  - `match_scope`: path class, artifact class, or semantic override
  - `rationale`
- **Validation rules**:
  - rules must be deterministic and non-contradictory
  - context-dependent rules must define the semantic trigger used to resolve
    them

## SemVer Decision Rule

- **Purpose**: Maps releasable changes to `patch`, `minor`, or `major`.
- **Core fields**:
  - `decision_id`
  - `required_bump`: `patch`, `minor`, or `major`
  - `trigger_type`: bug fix, additive capability, breaking contract, etc.
  - `evidence_source`: changed contract or behavior category
- **Validation rules**:
  - highest applicable bump wins
  - every releasable change must be explainable by at least one decision rule

## Governance Evaluation Result

- **Purpose**: Represents the machine-readable outcome of validating a pull
  request against release-governance rules.
- **Core fields**:
  - `overall_status`: `passed` or `failed`
  - `effective_classification`: `exempt` or `releasable`
  - `effective_bump`: `none`, `patch`, `minor`, or `major`
  - `missing_artifacts`: list of required artifacts not present
  - `mismatch_reasons`: list of bump or policy mismatches
  - `applied_rules`: identifiers of classification and decision rules used
- **Validation rules**:
  - failure reasons must be explicit and actionable
  - exempt results must not require version/changelog/fragment changes
  - releasable results must report the required bump even on failure

## Release-Preparation Change

- **Purpose**: Marks intentional metadata-only release work so it can be
  distinguished from accidental version/changelog drift.
- **Core fields**:
  - `declared`: boolean derived from `release_preparation: true`
  - `justification`: short human-readable reason
- **State transitions**:
  - absent → normal change evaluation
  - declared + metadata-only → allowed special case
  - declared + releasable deltas → still evaluated by normal highest-bump rules
