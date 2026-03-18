# Contract: Desired-State Repository Structure

**Audience**: Operators authoring desired state in Git

## Repository Expectations (MVP)

- A single repository defines desired workloads for one host.
- Quadlet unit files are stored under a dedicated top-level directory
  (e.g., `quadlets/`).
- Only supported Quadlet unit types are allowed in the repository.

## Validation Rules

- All Quadlet files MUST be syntactically valid.
- Duplicate unit names are rejected.
- Unsupported unit types or directives are rejected.
- Repository must include a stable revision identifier (commit hash or tag).

## Versioning

- The controller uses the repository revision as the desired state identifier.
- Changes are applied in commit order; no implicit rebase or rewrite handling.
