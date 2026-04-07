# Verification Behavior Inventory

This document inventories already implemented CoreOps behavior in verification-oriented terms rather than by module or crate layout. It is intended as a source document for backfilling accepted scenarios, drafting new candidate scenarios, and promoting real bug reproductions into permanent regression coverage.

Each behavior area captures:

- user-visible or operator-visible contract
- runtime semantics
- failure semantics
- output contract, when relevant
- revision or upgrade implications

Atomic contracts are annotated with:

- best verification level
- public-contract classification
- minimal proving scenario shape

Primary-home rule:

- Each contract should appear once in its primary behavior area.
- Later sections should narrow that contract to a more specific scope or rely on the earlier section by reference, rather than restating the same generic rule.
- Generic reconciliation-idempotency rules live under `Git-Defined Workload Convergence`.
- Generic convergence-success rules live under `Deterministic Three-Way Reconciliation and Rollback`.
- Public output-shape rules live under `Explainable Plan / Apply / Result / Explain Surfaces`.
- Harness-run classification and artifact-retention rules live under `VM-Backed End-to-End Verification Harness`.

`Shape` is intentionally not YAML. It is a compact proving sketch that answers:

- starting state
- trigger
- expected observation

Example:

- `Shape: clean VM + apply selected revision + verify converged`
- `Shape: converged VM + reapply same revision + expect no managed changes`
- `Shape: invalid override fixture + plan/apply + fail before side effects`

## 1. Git-Defined Workload Convergence

### Atomic Contracts

- `[accepted E2E now] [public operator contract]` CoreOps MUST accept a Git repository plus requested ref as the source of truth for managed host state. Shape: clean VM + apply selected revision from repo fixture + verify selected state converges.
- `[integration test] [internal semantic rule]` CoreOps MUST resolve the requested ref to an immutable revision for reconciliation semantics. Shape: request branch/tag selector + inspect resolved revision in plan/result output.
- `[integration test] [public operator contract]` `plan` MUST remain side-effect free. Shape: run plan twice + assert no managed files or status state changed.
- `[accepted E2E now] [public operator contract]` `apply` MUST materialize desired managed artifacts, reload systemd, and reconcile runtime state for the managed scope. Shape: clean VM + apply selected revision + verify rendered artifacts and running units.
- `[accepted E2E now] [public operator contract]` Reapplying the same resolved revision under materially unchanged host conditions MUST produce no managed changes. Shape: converged VM + reapply same revision + expect no managed changes.
- `[candidate E2E later] [public operator contract]` CoreOps MUST fail explicitly when repository input, desired-state layout, or supported managed artifacts are invalid. Shape: malformed repo fixture + plan/apply + fail clearly.
- `[accepted E2E now] [public operator contract]` CoreOps MUST NOT treat partial success as converged success. Shape: break one managed object during apply + verify run is not reported converged.
- `[integration test] [internal semantic rule]` Unknown or unsupported managed-scope inputs MUST NOT be silently adopted as valid desired state. Shape: unsupported artifact in fixture + evaluate/plan + reject it.
- `[integration test] [public automation contract]` Human-readable and machine-readable reconciliation surfaces SHOULD remain attributable to the resolved immutable revision under test. Shape: plan/apply JSON and humane output + compare revision attribution fields.

### User-visible / operator-visible contract

CoreOps accepts a Git repository and revision as the source of truth for managed host state. Operators can plan, apply, and re-apply that state to converge generated systemd and Quadlet-managed resources without ad hoc host edits becoming the normative source.

### Runtime semantics

- The desired state is selected from a repository plus requested ref and resolved to an immutable revision.
- Reconciliation is bounded to the supported managed resource kinds and managed directories.
- `plan` is side-effect free.
- `apply` materializes desired artifacts, reloads systemd, and reconciles unit runtime state.
- Re-applying unchanged desired state is intended to be idempotent.

### Failure semantics

- Invalid repository input, invalid desired-state layout, unsupported artifacts, or reconciliation-side runtime errors fail explicitly.
- Partial success is not silently treated as convergence.
- Unsupported or unknown managed-scope inputs are surfaced rather than implicitly coerced into managed behavior.

### Output contract

- Human-readable plan/apply surfaces report intended or executed changes.
- Machine-readable surfaces expose structured reconciliation data where supported by the public command contract.
- Audit and run context are emitted to the operator-facing reporting surfaces and journald when applicable.

### Revision / upgrade implications

- Desired-state behavior is revision-sensitive and must remain attributable to the resolved immutable revision under test.
- Repeated apply against the same revision and materially equivalent host conditions should converge with no unintended change set.
- Release changes that alter reconciliation semantics or public output need explicit version-policy review.

## 2. Unattended Agent-Driven Reconciliation

### Atomic Contracts

- `[candidate E2E later] [public operator contract]` CoreOps SHOULD support unattended reconciliation through a systemd-managed agent workflow. Shape: timer/oneshot-enabled VM + wait for agent-driven reconcile.
- `[integration test] [internal semantic rule]` Unattended reconciliation MUST use the same core reconciliation semantics as interactive execution. Shape: same fixture via agent boundary and interactive apply + compare semantic result model.
- `[integration test] [internal semantic rule]` Overlapping unattended runs for the same host scope MUST NOT proceed concurrently as if both were authoritative. Shape: simulate held agent lock + trigger second run + expect conflict.
- `[integration test] [public operator contract]` Journald SHOULD be the primary unattended audit sink under systemd execution. Shape: invoke agent boundary + inspect journald event path.
- `[candidate E2E later] [public operator contract]` Unattended failure MUST remain visible through systemd or journald surfaces rather than being silently retried to apparent success. Shape: failing agent run + inspect `systemctl status` and journal.
- `[integration test] [public automation contract]` Unattended reconciliation MUST preserve the same revision attribution model as manual reconciliation. Shape: same fixture via agent and manual apply + compare revision provenance.
- `[not stable enough yet] [provisional / not stable]` Controller upgrades MUST NOT create a semantic gap between unattended and interactive reconciliation outcomes. Shape: old controller then upgraded controller + compare unattended vs manual semantics.

### User-visible / operator-visible contract

CoreOps can run unattended through systemd as a oneshot agent plus timer, so the host can reconcile itself on a schedule without an operator invoking `apply` manually.

### Runtime semantics

- The agent runs through the same reconciliation model as interactive execution.
- Agent execution is serialized so overlapping runs do not proceed concurrently for the same host scope.
- Journald is the primary unattended audit sink.
- Ordering for managed artifacts remains deterministic and dependency-aware.

### Failure semantics

- Agent failure is explicit in systemd and journald rather than hidden behind silent retry loops.
- Lock or overlap conflicts prevent a second active run from proceeding as if both succeeded.
- Runtime reconciliation failure leaves an auditable failed outcome instead of reporting success.

### Output contract

- Systemd unit state and journald records are the main operator-facing outputs.
- Unattended execution preserves the same reconciliation outcome semantics as interactive runs.

### Revision / upgrade implications

- Scheduled reconciliation must preserve the same revision attribution as manual reconciliation.
- Upgrades must not create a semantic gap between operator-invoked runs and agent-driven runs.

## 3. Layered Desired-State Evaluation and Host Overrides

### Atomic Contracts

- `[integration test] [internal semantic rule]` Host identity MUST default to hostname unless explicitly overridden. Shape: evaluate same repo with implicit hostname and explicit override.
- `[accepted E2E now] [public operator contract]` Selected services MUST define the authoritative inclusion boundary for host desired state. Shape: clean VM + host selects one service set + verify only selected resources converge.
- `[unit test] [internal semantic rule]` Layering and override resolution MUST be deterministic. Shape: evaluate same layered inputs repeatedly + compare effective state byte-for-byte.
- `[integration test] [internal semantic rule]` CoreOps MUST evaluate layering into a concrete effective desired state before plan or apply side effects proceed. Shape: inspect evaluation/plan output before any side-effect boundary is invoked.
- `[integration test] [internal semantic rule]` Config payloads, drop-ins, and overlays MUST remain bounded to declared managed scope. Shape: fixture with out-of-scope overlay target + expect rejection.
- `[candidate E2E later] [public operator contract]` Invalid service selection, invalid override targets, and ownership-boundary violations MUST fail evaluation explicitly. Shape: invalid host override + plan/apply + fail before side effects.
- `[integration test] [internal semantic rule]` Conflicting or ambiguous effective state MUST NOT proceed to reconciliation as if valid. Shape: conflicting layered inputs + evaluation + stop before plan/apply.
- `[candidate E2E later] [public operator contract]` Managed config roots MUST behave as authoritative closed-world scopes where declared controller ownership applies. Shape: converged VM + add unmanaged file under managed root + reapply + expect removal.
- `[accepted E2E now] [public operator contract]` Reapplying unchanged layered desired state SHOULD remain idempotent after materialization. Shape: converged layered VM + reapply same revision + expect no managed changes.
- `[accepted E2E now] [public operator contract]` Layered config changes that affect dependent runtime behavior SHOULD produce deterministic downstream reconciliation effects. Shape: repo revision changes shared config + apply upgrade + verify expected dependent restart/update.

### User-visible / operator-visible contract

Operators can define reusable base services and combine them with host-specific selection and bounded overrides, yielding a concrete host desired state without duplicating whole service definitions.

### Runtime semantics

- Host identity defaults to hostname unless explicitly overridden.
- Selected services are the authoritative inclusion boundary for the host.
- Layering is deterministic and follows stable override ordering.
- Config payloads, drop-ins, and overlays are evaluated into a concrete desired state before plan/apply.
- Managed config roots are authoritative within their declared scope.

### Failure semantics

- Undefined service selection, invalid override targets, ownership-boundary violations, and invalid layered content fail evaluation before reconciliation side effects proceed.
- Conflicting or ambiguous effective state is rejected explicitly.
- Unmanaged residue inside closed managed roots is treated as drift/removal work, not silently preserved as part of desired state.

### Output contract

- Plans and results should make materialized overrides and resulting managed artifacts explainable.
- Effective-state reasoning must be deterministic enough for repeatable human and machine comparison.

### Revision / upgrade implications

- Small revision changes may produce large but deterministic materialized-state differences through layering.
- Upgrade and rollback behavior must preserve effective override ordering and ownership semantics.

## 4. Provenance and Canonical Status Tracking

### Atomic Contracts

- `[integration test] [public automation contract]` CoreOps MUST persist canonical reconciliation status in the runtime state directory. Shape: apply revision + inspect status file creation/update.
- `[integration test] [public automation contract]` `/var/lib/core-ops/status.json` MUST remain the canonical default status location unless explicitly changed by product policy. Shape: default-config run + inspect status path.
- `[integration test] [public operator contract]` `plan` MUST remain read-only with respect to canonical status state. Shape: snapshot status file + run plan + confirm unchanged.
- `[integration test] [public automation contract]` `apply` MUST update canonical status by default. Shape: clean VM + apply + inspect new status generation and result.
- `[integration test] [public automation contract]` Unattended reconciliation MUST update canonical status by default. Shape: trigger agent run + inspect status file after completion.
- `[unit test] [internal semantic rule]` Successful status publication MUST be atomic. Shape: write-through temp-and-rename path + assert no partial file state.
- `[integration test] [public automation contract]` Invalid or incomplete state snapshots MUST NOT be published as successful canonical state. Shape: force status write failure/corrupt candidate + confirm prior good state preserved.
- `[integration test] [public automation contract]` Never-run and in-progress conditions MUST be representable explicitly. Shape: inspect status before first run and during staged in-progress write path.
- `[integration test] [public automation contract]` Canonical status SHOULD distinguish requested selector context from resolved immutable revision context. Shape: apply branch-like ref + inspect requested ref vs resolved revision fields.
- `[integration test] [public automation contract]` Controller version, desired revision, and reconciliation outcome SHOULD remain attributable in canonical status. Shape: apply known revision + inspect version/revision/outcome fields.

### User-visible / operator-visible contract

CoreOps records canonical reconciliation provenance so operators can answer what revision was requested, what revision was resolved, what was attempted, what was last successfully applied, and what the current host status is.

### Runtime semantics

- Status is persisted under the canonical runtime state path, centered on `/var/lib/core-ops/status.json`.
- `plan` remains read-only.
- `apply` and unattended execution update canonical status by default.
- Successful status transitions are atomic and schema-governed.
- Reconciliation generation and run chronology are explicit rather than inferred from logs alone.

### Failure semantics

- Invalid or incomplete state snapshots must not be published as successful canonical state.
- In-progress and never-run conditions are explicit states, not implied by missing files alone.
- Failed runs may record diagnostic context without claiming successful application.

### Output contract

- Status is a user- and automation-consumable machine-readable contract.
- Provenance fields distinguish selector context from immutable revision context.
- Human-readable surfaces may render a summarized view derived from the same provenance data.

### Revision / upgrade implications

- Persisted state schema and field meanings are compatibility-sensitive.
- Operators must be able to distinguish controller-version changes from desired-revision changes.
- Upgrade work that changes provenance semantics or persisted state requires explicit compatibility review.

## 5. Native Mount-Backed Service Management

### Atomic Contracts

- `[accepted E2E now] [public operator contract]` CoreOps MUST support native `.mount` artifacts as first-class managed artifacts. Shape: clean VM + apply revision containing managed `.mount`.
- `[integration test] [internal semantic rule]` CoreOps MUST NOT require a non-native storage abstraction to manage host mounts. Shape: fixture with only native `.mount` metadata + evaluate accepted.
- `[accepted E2E now] [public operator contract]` Services that declare required mounts MUST NOT be treated as successfully runnable until those mounts are active and verified. Shape: clean VM + apply mount-backed service + verify mount comes up before service.
- `[candidate E2E later] [public operator contract]` CoreOps MAY support bounded `.automount` behavior for explicitly justified network-backed mounts. Shape: NFS-like fixture with explicit automount request + verify generated pair.
- `[integration test] [internal semantic rule]` CoreOps MUST NOT treat automount as the default for ordinary mount-backed services. Shape: ordinary mount fixture + inspect effective plan/artifacts for absence of automount.
- `[integration test] [internal semantic rule]` CoreOps MAY create the declared mountpoint path in a bounded way when policy allows it. Shape: missing mountpoint path + apply + inspect created target only.
- `[candidate E2E later] [public operator contract]` Invalid mount definitions, conflicting mount declarations, or invalid mountpoints MUST fail explicitly. Shape: conflicting mount fixtures + plan/apply + fail clearly.
- `[accepted E2E now] [public operator contract]` If a required mount is inactive or unverifiable, dependent service reconciliation MUST be blocked or degraded explicitly. Shape: unreachable mount source + apply + verify dependent service blocked.
- `[accepted E2E now] [public operator contract]` On desired-state removal of a managed mount, dependent managed services MUST be stopped before mount teardown proceeds. Shape: converged VM + remove mount from next revision + apply upgrade.
- `[candidate E2E later] [public operator contract]` Busy or unclean mount removal MUST fail explicitly rather than being forced silently. Shape: keep mount busy during removal revision + apply + fail explicitly.
- `[candidate E2E later] [public operator contract]` If a required mount disappears after prior health, CoreOps SHOULD leave the existing running service in place while preventing future starts or restarts until recovery. Shape: converged VM + break mount after service is up + reapply.
- `[accepted E2E now] [public operator contract]` Rebooted systems SHOULD preserve mount-backed service correctness under the same native dependency semantics claimed during normal reconciliation. Shape: converged mount-backed VM + reboot + verify mount and dependent service recover correctly.

### User-visible / operator-visible contract

Operators can declare native `.mount` artifacts, optionally paired with bounded `.automount` behavior for network-backed mounts, and bind selected services to those mounts using native systemd semantics instead of custom storage abstractions.

### Runtime semantics

- Mounts are first-class managed artifacts.
- Service-to-mount relationships are declared by selected services, not inferred from incidental runtime behavior.
- CoreOps may prepare the declared mountpoint path in a bounded way.
- Dependent services become runnable only after required mounts are active and verified.
- Removal is ordered safely: dependent services stop first, then mount teardown proceeds.

### Failure semantics

- Invalid mount definitions, conflicting mount declarations, invalid mountpoints, and unsupported automount usage fail explicitly.
- A required but inactive mount blocks or degrades dependent service reconciliation rather than being treated as success.
- Busy or unclean mount removal fails explicitly instead of being forced silently.
- A mount outage after prior health leaves the service relationship diagnosable and future starts/restarts blocked until recovery.

### Output contract

- Plan/apply/status output includes mount-specific diffs, activation decisions, dependency effects, and failure diagnostics.
- Managed mount identity is based on native unit naming, not a separate logical mount resource id.

### Revision / upgrade implications

- Revision changes can alter mount definitions, dependency wiring, and safe removal behavior.
- Compatibility-sensitive changes include mount semantics, operator-facing output, and persisted reconcile/provenance expectations.

## 6. Deterministic Three-Way Reconciliation and Rollback

### Atomic Contracts

- `[integration test] [internal semantic rule]` CoreOps MUST compute reconciliation decisions from desired state, last successfully applied state, and actual observed state. Shape: seed desired/last-applied/actual fixtures + inspect resulting plan decisions.
- `[unit test] [internal semantic rule]` CoreOps MUST normalize those inputs into canonical comparison forms. Shape: feed semantically equivalent variants + compare normalized output.
- `[integration test] [internal semantic rule]` Reconciliation MUST NOT rely solely on desired-versus-actual comparison when last-applied state is available for the same scope. Shape: fixture where last-applied changes the classification vs desired/actual only.
- `[unit test] [internal semantic rule]` Managed-object action classification MUST be deterministic for materially identical inputs. Shape: run same classification inputs repeatedly + compare actions.
- `[accepted E2E now] [public operator contract]` Execution order MUST be dependency-aware and deterministic. Shape: clean VM + apply dependency-ordered workload + inspect plan/apply ordering.
- `[integration test] [internal semantic rule]` Dependency cycles MUST fail explicitly before unsafe execution proceeds. Shape: construct cycle in effective state + plan + fail before side effects.
- `[integration test] [public operator contract]` Drift, stale residue, runtime variance, and expected desired change MUST remain distinguishable in reconciliation reporting. Shape: table-driven fixtures for each category + inspect plan/result causes.
- `[candidate E2E later] [public operator contract]` Rollback MUST be performed by selecting a previously successful retained revision and reconciling through the normal planner. Shape: converge revision A then B then request rollback to A.
- `[integration test] [public operator contract]` Rollback targets lacking sufficient retained normalized state MUST fail safely before execution. Shape: request rollback to expired or incomplete retained revision.
- `[integration test] [internal semantic rule]` Automatic retry for non-converging reconciliation MUST be bounded. Shape: persistent failure fixture + assert bounded retry count then stop.
- `[accepted E2E now] [public operator contract]` A revision MUST NOT be recorded as successfully applied until post-apply verification confirms convergence. Shape: force post-apply verification failure + inspect status/provenance.
- `[accepted E2E now] [public operator contract]` Partial application MUST record completed work, failed work, and remaining drift instead of reporting full convergence. Shape: multi-object apply where later object fails after earlier success.
- `Primary home: see 1. Git-Defined Workload Convergence for the generic no-managed-changes reapply contract.`

### User-visible / operator-visible contract

CoreOps plans from desired state, last successfully applied state, and actual observed state so operators can distinguish intended change from drift, stale residue, tolerated runtime variance, and rollback behavior.

### Runtime semantics

- Planning is based on canonical normalization of desired, last applied, and actual state.
- Each managed object is classified deterministically as create, update, replace, delete, no-op, blocked, or other supported action classes.
- Execution order is dependency-aware and deterministic.
- Rollback is performed by selecting a previously successful retained revision and reconciling through the normal planner.
- Retry for non-converging patterns is bounded.

### Failure semantics

- Dependency cycles, blocked prerequisites, rollback ineligibility, oscillation, and bounded retry exhaustion are first-class outcomes.
- A revision is not treated as successfully applied until post-apply verification confirms convergence.
- Partial application records what completed, what failed, and what drift remains.

### Output contract

- Structured diff output is authoritative for machine-readable planning and results.
- Human-readable views are renderings of the same underlying action and convergence data.
- Drift reporting identifies affected objects, drift categories, and intended controller response.

### Revision / upgrade implications

- Rollback eligibility depends on retained successful revision history.
- Changes to normalization rules, action semantics, or convergence classification can affect both behavior and compatibility.
- Deterministic planning for the same inputs is itself part of the operator-visible contract.

## 7. Explainable Plan / Apply / Result / Explain Surfaces

### Atomic Contracts

- `[integration test] [public automation contract]` CoreOps MUST preserve stable managed-object identity across plan, apply, result, and explain surfaces. Shape: same fixture + compare object ids across all views.
- `[integration test] [public operator contract]` Human-readable output MUST be a deterministic rendering of the same authoritative reconciliation data used for machine-readable output. Shape: same plan/result + compare humane rendering against JSON semantics.
- `[integration test] [public automation contract]` The machine-readable output model MUST remain authoritative for reconciliation semantics. Shape: assert humane output adds no semantics absent from JSON contract.
- `[integration test] [public operator contract]` Default human-readable plan output SHOULD emphasize changed or recovery-relevant objects while keeping unchanged scope discoverable. Shape: mixed-change fixture + inspect humane plan summary.
- `[integration test] [public operator contract]` Apply output MUST expose explicit progress states from pending through terminal result states. Shape: run apply fixture + inspect streamed progress events.
- `[integration test] [public operator contract]` Recovery-oriented actions MUST remain distinct from declarative updates when runtime recovery is required without desired-state change. Shape: unchanged desired state + broken runtime state + inspect plan action.
- `[accepted E2E now] [public operator contract]` Failed, blocked, skipped, no-op, and tolerated-variance outcomes MUST remain distinct in both live and final output. Shape: scenario with direct failure and downstream blockage + inspect report/result.
- `[integration test] [public operator contract]` Dependency-caused impact MUST be distinguishable from direct failure cause. Shape: failing prerequisite + inspect blocked dependent explanation.
- `[integration test] [public automation contract]` Public machine-readable field names, enum values, and documented ordering semantics MUST remain compatibility-sensitive. Shape: contract test against documented JSON fixture.
- `[integration test] [public operator contract]` Human-readable revision context SHOULD preserve the resolved immutable revision as primary and render requested-ref context only secondarily when meaningful. Shape: requested branch + resolved commit + inspect humane header.
- `[not stable enough yet] [provisional / not stable]` Output changes relied upon by users or automation MUST receive explicit compatibility review. Shape: compare proposed contract change against documented consumer-facing fields.

### User-visible / operator-visible contract

CoreOps exposes explainable reconciliation interfaces so operators and automation can inspect the plan, watch apply progress, inspect outcomes, and explain a single managed object using the same authoritative reconciliation model.

### Runtime semantics

- Stable object identity is preserved across plan, apply, result, and explain views.
- Human-readable output emphasizes changed or recovery-relevant objects while preserving access to unchanged scope.
- Apply progress moves objects through explicit pending, running, and terminal result states.
- Recovery-oriented actions are distinct from declarative updates when runtime variance exists without desired-state change.

### Failure semantics

- Failed, blocked, skipped, no-op, and tolerated-variance outcomes are distinct and explicitly reported.
- Dependency-caused impact is surfaced separately from direct failure cause.
- Incomplete or partially applied runs are not narrated as converged success.

### Output contract

- The machine-readable output model is authoritative.
- Human-readable output is a deterministic rendering of that model.
- Compatibility-sensitive contract surfaces include field names, enum values, and documented ordering semantics.
- Revision context preserves both requested selector context and resolved immutable revision context where available.

### Revision / upgrade implications

- Output-schema changes are compatibility-sensitive for users, automation, and agentic consumers.
- Requested ref versus resolved revision semantics must remain stable across upgrades.
- Replacing the prior plan JSON contract was itself a compatibility event and future changes should be evaluated at the same level.

## 8. VM-Backed End-to-End Verification Harness

### Atomic Contracts

- `[accepted E2E now] [public operator contract]` Accepted end-to-end scenarios MUST execute against disposable VM-backed environments. Shape: accepted scenario + real VM-backed run.
- `[integration test] [internal semantic rule]` Synthetic or non-VM execution MUST NOT be treated as satisfying accepted end-to-end verification on its own. Shape: inspect mode/backend gating semantics in run result.
- `[integration test] [internal semantic rule]` Scenario definitions MUST separate behavioral intent, environment selection, and harness-policy configuration. Shape: parse scenario fixture using profiles plus overrides.
- `[integration test] [public automation contract]` Verification runs MUST use unique run identifiers and isolated workspaces. Shape: run same scenario twice + compare run ids and bundle paths.
- `[accepted E2E now] [public automation contract]` Scenario outcomes MUST classify failure as assertion failure, infrastructure failure, timeout, or harness error. Shape: induce one case of each failure class across fixtures.
- `[accepted E2E now] [public automation contract]` Verification runs MUST always retain the core artifact bundle required for offline diagnosis. Shape: any run + inspect artifact bundle contents.
- `[accepted E2E now] [public automation contract]` Failed runs SHOULD retain additional failure-specific diagnostics. Shape: failing VM-backed run + inspect failure-specific artifacts.
- `[accepted E2E now] [public operator contract]` Default execution MUST tear down disposable environments after artifact capture completes. Shape: default run + confirm VM absent after completion.
- `[integration test] [public operator contract]` Debug execution MAY retain or pause before teardown for operator investigation. Shape: debug run with retain or pause flag + inspect lifecycle behavior.
- `[integration test] [public automation contract]` Repeated execution of the same accepted scenario SHOULD preserve the same meaningful outcome shape while producing distinct run identifiers and bundles. Shape: run same accepted scenario twice + compare stable semantics.
- `[integration test] [public automation contract]` Non-interactive execution MUST emit machine-readable results and deterministic exit semantics suitable for gating. Shape: `--ci --json` run + inspect exit code and JSON.
- `[integration test] [public automation contract]` Batch verification runs MUST preserve both batch-level revision-selection context and per-scenario revision-under-test provenance. Shape: accepted corpus run + inspect top-level and per-scenario provenance.
- `[integration test] [public automation contract]` Single-VM execution MUST remain the authoritative supported topology for the current release line. Shape: attempt unsupported topology or inspect scenario validation constraints.

### User-visible / operator-visible contract

CoreOps provides a dedicated verification entrypoint that runs declarative scenarios against disposable VM-backed environments to validate real runtime behavior, not just synthetic or in-process behavior.

### Runtime semantics

- VM-backed disposable-machine execution is the authoritative verification mode.
- Scenarios are declarative and separate behavioral intent, environment selection, and harness-policy overrides.
- Supported semantic steps include boot, readiness, CoreOps commands, guest commands, runtime mutation, and reboot.
- Accepted scenarios form the maintained gating corpus; generated candidates remain advisory until reviewed.
- Runs are isolated by unique run ID and artifact bundle.

### Failure semantics

- Runs classify failure as assertion failure, infrastructure failure, timeout, or harness error.
- Failed runs preserve offline diagnostics sufficient for later investigation.
- Default execution tears down the environment after artifact capture; debug mode may retain or pause before teardown.
- Unsupported infrastructure or unsupported guest topology is rejected rather than treated as partial feature support.

### Output contract

- Non-interactive execution emits machine-readable run results and deterministic exit semantics suitable for gating.
- Artifact bundles always retain core run evidence and collect failure-specific enrichment when needed.
- Batch runs preserve scenario-level and batch-level revision provenance.

### Revision / upgrade implications

- Accepted regression scenarios can be promoted from real bug reproductions and retained permanently.
- Verification outputs and exit semantics are part of the public operational contract when relied upon by users or automation.
- Corpus coverage should evolve with feature behavior, including upgrade transitions, reboot resilience, drift correction, idempotency, and failure diagnosis.

## 8.1 Serial-Console Guest Readiness

### Atomic Contracts

- `[integration test] [public automation contract]` VM-backed verification MUST accept the first valid current-run serial-console readiness record as the authoritative guest IPv4 source. Shape: console log with stale lines then valid current-run record + verify first valid IPv4 wins.
- `[integration test] [internal semantic rule]` Serial-console readiness MUST reject stale, mismatched, and malformed records without unblocking the run. Shape: previous-run log replay + malformed current-run record + later valid record.
- `[integration test] [public operator contract]` Missing valid readiness within the configured window MUST end as an explicit readiness timeout distinct from behavioral CoreOps failure. Shape: env-backed scenario + no valid readiness record + inspect timeout outcome and readiness evidence.
- `[integration test] [public operator contract]` Readiness rejection before guest access MUST surface as infrastructure-style readiness failure distinct from behavioral CoreOps failure. Shape: only malformed readiness records + inspect failure summary and machine-readable run payload.
- `[integration test] [public automation contract]` Migration-only ARP fallback MUST remain subordinate to a valid accepted readiness record. Shape: valid readiness record present + fallback enabled + verify serial-console source remains authoritative.

### User-visible / operator-visible contract

The verification harness learns guest reachability from the guest itself by
consuming a run-scoped readiness record on the serial console. Operators can
tell from retained artifacts and run outputs whether readiness was accepted,
rejected, timed out, or fell back during migration.

### Runtime semantics

- The first valid current-run readiness record wins.
- Later matching records are ignored for guest selection and retained only as diagnostics.
- Rejected stale and malformed records do not advance guest readiness state.
- ARP-based discovery is an opt-in migration fallback rather than the primary contract.

### Failure semantics

- No valid readiness record before the deadline is a readiness timeout.
- Malformed or mismatched readiness data is rejected explicitly.
- Readiness acquisition failures remain distinct from later behavioral CoreOps failures.

### Output contract

- `readiness-evidence.json` captures accepted and rejected records plus final status.
- Human-readable and machine-readable verification outputs surface readiness status separately from behavioral failure summaries.

### Revision / upgrade implications

- Marker, required fields, failure-class semantics, and readiness artifact shape are compatibility-sensitive.

## 9. Spec-Driven Candidate Scenario Generation

### Atomic Contracts

- `[integration test] [internal semantic rule]` Feature specifications MUST remain the canonical semantic source for candidate scenario generation. Shape: generate candidates from feature spec + inspect derived claims.
- `[integration test] [internal semantic rule]` Participating feature specifications MUST include mandatory verification guidance. Shape: spec missing guidance + generation should fail.
- `[integration test] [internal semantic rule]` Verification guidance MUST support generation without becoming a rigid substitute for the full feature specification. Shape: sparse guidance with rich spec text + generation still succeeds meaningfully.
- `[integration test] [public operator contract]` Generated candidates MUST declare behavioral claim, rationale, and coverage class. Shape: candidate generation + inspect emitted metadata fields.
- `[integration test] [internal semantic rule]` Candidate validation MUST reject malformed, redundant, unstable, unsupported, or assertion-free scenarios. Shape: submit bad candidate fixtures + inspect rejection reasons.
- `[integration test] [public automation contract]` Candidate scenarios MUST NOT gate CI or release workflows until explicitly accepted. Shape: corpus run containing candidate fixture + verify it is ignored for gating.
- `[candidate E2E later] [public operator contract]` Accepted scenarios SHOULD become the maintained regression corpus for repeated execution. Shape: review candidate into accepted corpus + rerun as accepted scenario.
- `[not stable enough yet] [provisional / not stable]` Spec changes SHOULD drive candidate generation or corpus updates where operator-visible behavior changes. Shape: modify feature spec behavior text + regenerate candidates and compare coverage.

### User-visible / operator-visible contract

CoreOps can derive candidate verification scenarios from feature specifications so verification starts from declared behavior rather than only from hand-authored tests.

### Runtime semantics

- Feature specifications are the canonical semantic source.
- Verification guidance inside specs is mandatory for participating features, but it is supporting guidance rather than a rigid intermediate representation.
- Generated candidates must declare behavioral claim, rationale, and coverage class.
- Candidate validation rejects malformed, redundant, unstable, or unsupported scenarios before they enter the accepted corpus.

### Failure semantics

- Missing required verification guidance, unsupported scenario shape, or insufficient deterministic assertions is a generation-time failure.
- Candidate scenarios do not gate CI or release decisions until explicitly accepted.

### Output contract

- Candidate scenarios are durable artifacts that can be reviewed, refined, accepted, or discarded.
- Coverage classification and scenario intent are part of the operator/author-facing contract for generated candidates.

### Revision / upgrade implications

- Spec changes should naturally drive new candidate scenarios or corpus updates.
- Verification guidance fields are mandatory for participating feature specs and therefore part of the repo authoring contract.

## 10. Backfill-Oriented Scenario Classes To Cover

The existing implementation implies at least the following scenario classes should be backfilled across the behavior inventory above:

- convergence of unchanged desired state
- idempotent re-apply of an already converged revision
- drift detection and correction
- dependency ordering and blocker visibility
- reboot resilience
- revision transition and rollback behavior
- failure diagnosis with retained artifacts
- partial apply, blocked apply, and recovery
- explain / plan / apply consistency
- unattended agent execution semantics
- mount-backed service behavior
- provenance and status persistence
- public command-surface compatibility where documented or relied upon

## Suggested Use For Corpus Backfill

- Use one accepted scenario per stable, operator-visible contract that should gate regressions.
- Use repository-evolution fixtures when the behavior inherently spans revisions, rollback, or upgrade transitions.
- Use candidate scenarios for newly identified coverage before reviewing them into the accepted corpus.
- Use regression scenarios for real bugs that reproduced against the VM-backed harness and should remain permanently runnable.

## Coverage Priority Sort

This sort is intended to drive immediate corpus backfill. It prioritizes behaviors that most directly exercise:

- fresh convergence
- idempotent re-apply
- drift detection and correction
- dependency ordering
- partial failure classification
- reboot resilience where relevant

### Must Have Accepted E2E Coverage Now

#### Git-defined workload convergence

- Fresh convergence of a selected revision from a clean disposable VM.
- Idempotent re-apply of the same revision with no unintended changes.
- Drift detection and correction for managed artifacts after external mutation.
- Explicit failure classification when reconcile cannot converge due to desired-state or runtime errors.

Why now:
- This is the core operator-visible contract of CoreOps.
- It directly covers fresh convergence, idempotency, drift correction, and failure classification.

#### Layered desired-state evaluation and host overrides

- Fresh convergence of layered base plus host-specific effective state.
- Idempotent re-apply after effective-state materialization.
- Dependency ordering where layered config changes force dependent runtime changes.
- Explicit failure on invalid selected services, invalid overrides, or ownership-boundary violations.

Why now:
- Layering defines what CoreOps is actually supposed to reconcile.
- It is central to realistic desired-state behavior and deterministic drift correction.

#### Native mount-backed service management

- Fresh convergence of a mount-backed service from clean boot.
- Dependency ordering: mount active and verified before dependent service becomes runnable.
- Drift or outage correction when a required mount disappears or recovers.
- Partial failure classification when mount activation fails and dependent service is blocked.
- Reboot resilience for mounts and dependent services where the feature claims native boot/runtime semantics.

Why now:
- This is one of the clearest dependency-ordering and partial-failure surfaces in the product.
- It has explicit operator-visible runtime and recovery semantics.

#### Deterministic three-way reconciliation and rollback-facing behavior

- Fresh convergence to a target revision with converged post-apply verification.
- Idempotent re-apply when desired, last-applied, and actual state materially match.
- Drift detection and correction with clear category/explanation.
- Partial failure classification for blocked, oscillating, or non-converging object sets.

Why now:
- These semantics are foundational for trustworthy apply behavior.
- They define the meaning of convergence and non-convergence.

#### VM-backed verification harness itself

- Fresh execution of an accepted scenario on a disposable VM.
- Idempotent re-run of the same accepted scenario with stable meaningful outcome shape.
- Infrastructure failure vs assertion failure vs timeout classification.
- Reboot-resilient scenario execution when a scenario includes reboot.

Why now:
- The harness is now part of the repo’s standing verification contract.
- It is the mechanism used to validate the rest of the accepted corpus.

### Should Have Coverage Soon

#### Unattended agent-driven reconciliation

- Successful unattended convergence from timer-driven execution.
- Failure classification for unattended failed runs.
- Drift correction when unattended execution detects changed managed state.

Why soon:
- Operator-visible and important, but the interactive reconcile path is the more immediate regression gate.
- Some semantics are better exercised once the base accepted corpus is in place.

#### Provenance and canonical status tracking

- Correct status update after successful fresh convergence.
- Correct status after failed reconcile or never-run/in-progress transitions.
- Revision attribution consistency across requested ref, resolved revision, and last applied context.

Why soon:
- Stable and important for automation and diagnosis.
- Often best paired with accepted E2E scenarios that already exercise converge/fail flows.

#### Explainable plan / apply / result / explain surfaces

- Stable explanation of changed vs unchanged objects for representative reconcile cases.
- Public command-surface coverage for supported humane and machine-readable outputs.
- Failure reporting that distinguishes failed, blocked, skipped, and tolerated-variance outcomes.

Why soon:
- Important public contract, especially for automation.
- Some of this should gate regressions, but not every rendering detail needs immediate accepted E2E coverage.

#### Spec-driven candidate scenario generation

- Generation rejection for malformed or underspecified candidate inputs.
- Acceptance workflow expectations for candidate vs accepted corpus.

Why soon:
- Important for corpus growth, but not as central to runtime correctness as reconciliation behaviors themselves.

### Better Handled By Integration / Unit Tests

#### Fine-grained output-shape and schema stability checks

- Exact machine-readable field sets, enum values, and ordering semantics.
- Humane rendering details that do not require a real VM to validate.

Why here:
- These are high-signal contract checks but are cheaper and more deterministic in integration tests than full E2E.

#### Internal normalization and classification rules

- Drift category precedence.
- Action classification precedence.
- Outcome classification precedence.
- Deterministic ordering rules for plan entries and result summaries.

Why here:
- These are semantic core rules that benefit from exhaustive table-driven tests more than from broad E2E coverage.

#### Candidate-generation validation rules

- Missing verification guidance rejection.
- Duplicate candidate detection.
- Unsupported scenario-shape rejection.

Why here:
- These are mostly pure validation behaviors and are best kept deterministic and fast.

#### Pause-before-teardown and debug-only workflow constraints

- Flag compatibility constraints.
- Interactive guardrails and debug-policy enforcement.

Why here:
- Important, but mostly CLI/workflow semantics rather than core runtime behavior.

### Not Stable Enough Yet To Spec Tightly

#### Exhaustive command-surface coverage across every command and flag

- Full CLI surface parity beyond the currently documented public contract.

Why not yet:
- The spec intentionally narrows v1 command-surface verification to important public outputs and behaviors.

#### Broad unattended operational policy details

- Exact timer cadence policy.
- Long-horizon retry strategies beyond bounded reconcile semantics.

Why not yet:
- The product semantics are stable, but policy-level defaults are not the best current E2E backfill target.

#### Fleet-like or multi-node verification behavior

- Coordinated multi-host or distributed rollback semantics.

Why not yet:
- Explicitly out of scope for the current single-node, single-VM model.

## Immediate Accepted-Corpus Backfill Order

If you are backfilling the corpus now, prioritize accepted E2E scenarios in this order:

1. Fresh convergence of a representative layered service revision on a clean VM.
2. Idempotent re-apply of the same revision with no managed changes.
3. Drift detection and correction after external mutation of managed state.
4. Dependency ordering for mount-backed or otherwise prerequisite-bound services.
5. Partial failure classification where one failed prerequisite blocks downstream work.
6. Reboot resilience for scenarios whose runtime contract depends on restart survival.
7. Provenance/status confirmation paired with the successful and failed runs above.

## Accepted-Corpus Minimum Set

This section considers only contracts marked `[accepted E2E now]`.

### Shared Proving Shapes

#### Clean VM + apply selected revision + verify converged

Covers contracts that prove:

- selected revision is the source of truth
- apply materializes and reconciles managed state
- selected services define the host scope
- native mount artifacts are first-class when present
- required mounts become active before dependent services are treated as runnable
- accepted end-to-end scenarios execute on real disposable VMs
- default runs retain the core artifact bundle
- default runs tear down the VM after artifact capture

#### Converged VM + reapply same revision + expect no managed changes

Covers contracts that prove:

- generic idempotent re-apply
- layered desired-state idempotency after materialization

#### Repo revision changes + apply upgrade + verify deterministic downstream effect

Covers contracts that prove:

- layered config changes propagate deterministic downstream runtime effects
- desired-state removal of a managed mount stops dependents before teardown

#### Converged mount-backed VM + reboot + verify recovery

Covers contracts that prove:

- mount-backed service semantics survive reboot under the declared native dependency model

#### Failing run with direct failure and downstream blockage

Covers contracts that prove:

- partial success is not reported as convergence
- partial application records completed work, failed work, and remaining drift
- failed, blocked, skipped, no-op, and tolerated-variance outcomes remain distinct
- failed runs retain failure-specific diagnostics
- failure outcomes are classified correctly for gating

#### Failing run due to inactive required mount

Covers contracts that prove:

- a required mount failure blocks or degrades the dependent service explicitly
- dependency ordering is enforced at runtime, not only in planning

#### Failing run after post-apply verification does not converge

Covers contracts that prove:

- a revision is not recorded as successfully applied until post-apply verification confirms convergence

#### Timeout / infrastructure-failure harness runs

Covers contracts that prove:

- timeout and infrastructure-failure classes are distinct machine-readable outcomes

### Proposed Smallest Accepted Scenario Set

Implementation note:

- Under the current `008` harness, scenarios do not hand live VMs to later scenarios.
- Any proof shape that requires a converged VM as input must therefore be expressed as a single multi-step scenario.

#### 1. `accepted-layered-convergence-idempotency`

Shape:
- clean VM
- apply representative layered selected revision
- verify only selected resources converge
- reapply same resolved revision
- assert no managed changes
- inspect artifact bundle
- allow normal teardown

Covers:
- CoreOps MUST accept a Git repository plus requested ref as the source of truth for managed host state.
- `apply` MUST materialize desired managed artifacts, reload systemd, and reconcile runtime state for the managed scope.
- Selected services MUST define the authoritative inclusion boundary for host desired state.
- Reapplying the same resolved revision under materially unchanged host conditions MUST produce no managed changes.
- Reapplying unchanged layered desired state SHOULD remain idempotent after materialization.
- Accepted end-to-end scenarios MUST execute against disposable VM-backed environments.
- Verification runs MUST always retain the core artifact bundle required for offline diagnosis.
- Default execution MUST tear down disposable environments after artifact capture completes.

#### 2. `accepted-layered-upgrade-transition`

Shape:
- clean VM
- apply base revision
- advance to next revision with layered shared-config change
- verify expected dependent restart or update only

Covers:
- Layered config changes that affect dependent runtime behavior SHOULD produce deterministic downstream reconciliation effects.

#### 3. `accepted-mount-convergence-reboot`

Shape:
- clean VM
- apply mount-backed service revision
- verify mount active before dependent service is treated as runnable
- inspect dependency ordering in plan/apply evidence
- reboot guest
- verify mount-backed service recovers correctly

Covers:
- CoreOps MUST support native `.mount` artifacts as first-class managed artifacts.
- Services that declare required mounts MUST NOT be treated as successfully runnable until those mounts are active and verified.
- Execution order MUST be dependency-aware and deterministic.
- Rebooted systems SHOULD preserve mount-backed service correctness under the same native dependency semantics claimed during normal reconciliation.

#### 4. `accepted-mount-blocked-failure`

Shape:
- clean VM
- apply revision with unreachable required mount source
- verify dependent service is blocked or degraded
- inspect report and retained failure diagnostics

Covers:
- If a required mount is inactive or unverifiable, dependent service reconciliation MUST be blocked or degraded explicitly.
- CoreOps MUST NOT treat partial success as converged success.
- Failed, blocked, skipped, no-op, and tolerated-variance outcomes MUST remain distinct in both live and final output.
- Scenario outcomes MUST classify failure as assertion failure, infrastructure failure, timeout, or harness error.
- Failed runs SHOULD retain additional failure-specific diagnostics.

#### 5. `accepted-mount-removal-ordering`

Shape:
- clean VM
- apply converged mount-backed revision
- move to next revision that removes the managed mount
- apply upgrade
- verify dependent service stops before mount teardown

Covers:
- On desired-state removal of a managed mount, dependent managed services MUST be stopped before mount teardown proceeds.

#### 6. `accepted-partial-apply-verification-failure`

Shape:
- clean VM
- apply multi-object revision where early work succeeds but a later object or final verification fails
- inspect status/provenance and final report

Covers:
- A revision MUST NOT be recorded as successfully applied until post-apply verification confirms convergence.
- Partial application MUST record completed work, failed work, and remaining drift instead of reporting full convergence.

#### 7. `accepted-timeout-classification`

Shape:
- clean VM
- execute scenario with deliberately unreachable readiness or command timeout
- verify timeout outcome and retained core artifacts

Covers:
- Scenario outcomes MUST classify failure as assertion failure, infrastructure failure, timeout, or harness error.

#### 8. `accepted-infrastructure-failure`

Shape:
- VM-backed run with induced guest provisioning, staging, or command-boundary infrastructure failure
- verify infrastructure-failure outcome and retained core artifacts

Covers:
- Scenario outcomes MUST classify failure as assertion failure, infrastructure failure, timeout, or harness error.

### Why This Is The Smallest Practical Accepted Set

- Scenario 1 combines fresh convergence and idempotent re-apply because the current harness cannot hand a live converged VM from one scenario to another.
- Scenario 2 isolates deterministic upgrade behavior without forcing mount-specific semantics into every upgrade case.
- Scenario 3 combines mount first-class support, dependency ordering, and reboot resilience into one scenario.
- Scenario 4 combines blocked dependency failure, non-convergence classification, distinct live/final outcomes, and failure-specific artifact retention.
- Scenario 5 keeps destructive mount-removal ordering separate because it is a different transition shape from mount activation and also must self-bootstrap its converged precondition.
- Scenario 6 isolates the “successful apply record must wait for post-apply verification” rule from earlier direct dependency failures.
- Scenarios 7 and 8 are both needed because timeout and infrastructure failure are distinct public outcome classes and should not be collapsed into one failing scenario.

### Contracts Left Out Of This Accepted-Corpus Pass

The following verification levels are intentionally excluded from this accepted-corpus proposal:

- `[candidate E2E later]`
- `[integration test]`
- `[unit test]`
- `[not stable enough yet]`
