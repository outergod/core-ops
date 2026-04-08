# Data Model: Distribution Readiness

## EntryPointSurface

**Purpose**: Represents the stable public project entrypoint for outside
evaluators.

**Fields**
- `framing_statement`: concise statement of what CoreOps is for
- `goals`: list of supported goals
- `non_goals`: list of explicitly unsupported problem classes
- `target_audience`: intended user groups
- `support_boundary`: reference to supported, untested, and unsupported system
  classes
- `authorship_disclosure`: reference to AI authorship statement and guarantee
  explanation
- `trust_story`: reference to what CoreOps modifies, auditability, and reversal
- `credibility_surface`: embedded status block with stable placement
- `installation_path_refs`: links to supported install flow(s)
- `verification_flow_ref`: link to operator-facing verification flow
- `license_ref`: link to AGPLv3+ declaration
- `code_of_conduct_ref`: link to community behavior document
- `logo_surface`: logo asset or reserved placeholder

**Validation Rules**
- Must be locatable from the project entrypoint.
- Must include all public contract references required by the spec.
- Credibility values must remain consistently locatable across releases.

## CredibilitySurface

**Purpose**: Provides a compact public status snapshot for outside evaluators.

**Fields**
- `latest_release_identity`: current release version and release locator
- `release_gate_status`: pass/fail state of the current release gate
- `accepted_verification_status`: current accepted verification result
- `artifact_availability`: currently published binary artifacts
- `location_contract`: stable entrypoint section or anchor used across releases
- `updated_at`: last refreshed timestamp or release association

**Validation Rules**
- Must appear in the project entrypoint.
- Must match the same release identity and gate state as underlying release
  materials.
- Must remain structurally stable enough to locate the same values across
  releases.

## DistributionArtifact

**Purpose**: Describes the public binary form distributed to operators.

**Fields**
- `artifact_name`
- `artifact_type`: binary archive, raw binary, or similar direct-consumption form
- `target_system_class`: supported host class for this artifact
- `download_location`
- `checksum_or_integrity_ref`
- `release_identity_ref`
- `license_ref`

**Validation Rules**
- Must support direct manual consumption.
- Must correspond to the documented installation path.
- Must map to one release identity.

## InstallationPath

**Purpose**: Defines the deterministic operator installation sequence.

**Fields**
- `artifact_acquisition_step`
- `installation_step`
- `first_command_step`
- `smoke_test_step`
- `documented_prerequisites`
- `external_dependencies`
- `supported_system_class`
- `cold_start_expectation`

**Validation Rules**
- Must document acquisition, installation, first command, and smoke-test
  validation.
- Should minimize external dependencies.
- Must not depend on undeclared prerequisites or hidden system knowledge.
- Must succeed on a freshly provisioned supported environment.

## VerificationFlow

**Purpose**: Defines the minimal operator-facing flow for verifying correct
behavior on a user-managed system.

**Fields**
- `entry_condition`: installed runnable CoreOps instance
- `observable_check`: explicit state change or convergence check
- `expected_outcome`
- `reproducibility_scope`: supported system class where the result should
  reproduce
- `failure_guidance`
- `artifacts_or_outputs`: humane or machine-readable evidence used by operators

**Validation Rules**
- Must not rely on internal test infrastructure.
- Must include at least one reproducible expected observable outcome.
- Must succeed on a freshly provisioned supported environment.

## ReleaseIdentity

**Purpose**: Tracks the public identity of a distributed build.

**Fields**
- `binary_version`
- `source_revision`
- `spec_context`
- `release_label`
- `visibility_surfaces`: CLI, logs, explain/report surfaces, release materials

**Validation Rules**
- Must remain consistent across declared visibility surfaces.
- Must be attributable to each distributed build.

## ReleaseGate

**Purpose**: Models the decision contract for treating a candidate as
distribution-ready.

**Fields**
- `build_result`
- `accepted_verification_result`
- `spec_conformance_result`
- `determinism_result`
- `verification_environment_ref`
- `decision`
- `failure_reasons`

**Validation Rules**
- Decision is `ready` only if all required checks pass.
- Re-running under materially unchanged inputs must yield the same decision.

## VerificationEnvironmentIdentity

**Purpose**: Describes the authoritative environment used for release-gating
verification.

**Fields**
- `environment_name`
- `system_class`
- `runner_definition_ref`
- `version_marker`
- `reproducibility_notes`
- `drift_detection_basis`

**Validation Rules**
- Must be documented and discoverable from maintained project materials.
- Must be versioned sufficiently to detect drift over time.
- Must identify the environment used by a release-gate result.

## SupportBoundary

**Purpose**: Defines what environments and usage modes are supported.

**Fields**
- `officially_supported`: Fedora CoreOS
- `theoretically_compatible_but_untested`: other systemd-based hosts
- `unsupported`: non-systemd environments, containerized CoreOps execution,
  excluded orchestration/configuration-management roles
- `rationale`

**Validation Rules**
- Must be visible from the entrypoint.
- Must not blur unsupported and supported classes.

## LicenseDeclaration

**Purpose**: Defines the governing public software license.

**Fields**
- `license_name`: GNU Affero General Public License version 3 or later
- `scope`: source and released artifacts
- `discovery_surface`

**Validation Rules**
- Must be visible from published project materials.
- Must remain consistent across source and distribution surfaces.

## CodeOfConduct

**Purpose**: Defines the public community behavior document.

**Fields**
- `document_location`
- `entrypoint_reference`
- `audience`: contributors and users

**Validation Rules**
- Must be discoverable from the project entrypoint.

## ChangelogRecord

**Purpose**: Tracks externally relevant change history for releases.

**Fields**
- `release_identity_ref`
- `change_categories`
- `operator_visible_impacts`
- `compatibility_notes`
- `document_format`: Keep a Changelog

**Validation Rules**
- Must preserve continuity across releases.
- Must record externally relevant changes for each distributed build.
