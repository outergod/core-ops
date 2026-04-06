# Development (Nix + direnv)

This project assumes a Nix shell provided by `shell.nix` and loaded via direnv.
Do not assume Rust tooling is installed globally.

## Setup

1. Install direnv and Nix.
2. Run `direnv allow` at repo root.

## Common Commands

- Format: `cargo fmt` (or `make fmt`)
- Lint: `cargo clippy --all-targets -- -D warnings` (or `make lint`)
- Test: `cargo test` (or `make test`)

Assume the nix shell is already active, and do not run commands via `direnv exec`.

Rust changes are not considered complete until both `cargo test` and
`cargo clippy --all-targets -- -D warnings` pass, unless a temporary exception
is explicitly documented with follow-up work.

## Systemd Agent Configuration

The CoreOps host agent is designed to run as a oneshot service triggered by a timer.
Use a systemd drop-in to configure the repo source and revision without editing
unit files in place. The contract units are named `core-ops.service` and
`core-ops.timer`.

```
systemctl edit core-ops.service
```

Suggested drop-in content:

```
[Service]
Environment=CORE_OPS_REPO=ssh://git@github.com/your-org/quadlets.git
Environment=CORE_OPS_REV=main
Environment=CORE_OPS_QUADLET_DIR=/etc/containers/systemd
Environment=CORE_OPS_SYSTEMD_UNIT_DIR=/etc/systemd/system
```

Apply changes with:

- `systemctl daemon-reload`
- `systemctl restart core-ops.service`

Timer enablement example:

```
systemctl enable --now core-ops.timer
```

## Layered Overrides Development

Use the layered overrides fixture in `tests/fixtures/layered_overrides/` for
local testing. The repository layout should include:

- `services/<service>/` for base artifacts and base drop-ins
- `hosts/<host>/host.yaml` with explicit service selection
- `hosts/<host>/overrides/` for host-specific drop-ins

Override host selection during development with:

```
CORE_OPS_HOST=<host> core-ops plan --repo <repo> --rev <rev>
```

When adding or changing behavior, ensure tests and diagnostics preserve
machine-readable provenance for both the `core-ops` binary revision and the
desired-state revision being reconciled.

Any change that affects externally observable behavior, persisted state schema,
CLI output, reconciliation semantics, or compatibility must evaluate and
update the release version policy. The canonical controller version is the
package version in `Cargo.toml`.

## Verification Harness Workflow

`core-ops-verify` is the dedicated development and CI entrypoint for the E2E
verification harness. Its public execution path is VM-backed by default.
Synthetic execution remains available only as hidden internal test support and
is not the intended path for developer signoff, CI gating, or release
verification.

### Execution Modes

- Single-scenario execution uses `--scenario <path>`.
- Accepted-corpus CI execution uses `--accepted-dir <dir> --ci`.
- Focused accepted-corpus reruns use repeated `--scenario-id <id>` values with
  `--accepted-dir`.
- `--debug` retains the disposable VM after artifact collection.
- `--json` emits the authoritative machine-readable `verification_run`
  contract on stdout.

Examples:

```bash
cargo run --bin core-ops-verify -- run \
  --scenario tests/fixtures/verification/scenarios/minimal-accepted.yaml

cargo run --bin core-ops-verify -- run \
  --scenario tests/fixtures/verification/scenarios/minimal-accepted.yaml \
  --debug --json

cargo run --bin core-ops-verify -- run \
  --accepted-dir tests/fixtures/verification/scenarios \
  --ci --json
```

### Hypervisor Selection

If no libvirt override is set, the verification harness uses the local libvirt
system URI (`qemu:///system`). This is the default when the same machine acts
as both CLI host and hypervisor.

To run against a remote hypervisor, set either:

- `CORE_OPS_VERIFY_VM_HOST=<host>`
  This derives the common remote URI shape
  `qemu+ssh://core@<host>/system`.
- `CORE_OPS_VERIFY_LIBVIRT_URI=<uri>`
  This fully overrides the libvirt connection target and takes precedence over
  `CORE_OPS_VERIFY_VM_HOST`.

VM-backed runs normally also need a guest-runnable CoreOps binary:

```bash
export CORE_OPS_VERIFY_VM_HOST=ulthar
export CORE_OPS_VERIFY_CORE_OPS_BIN=target/debug/core-ops
```

### Repository-Evolution and Regression Workflow

Accepted scenarios may reference repository-history fixtures rather than a
single static checkout. This allows a scenario to exercise realistic revision
transitions, bug reproductions, and regression reruns.

- Repository-history fixtures live under
  `tests/fixtures/verification/repos/`.
- Accepted scenario definitions live under
  `tests/fixtures/verification/scenarios/`.
- Generated candidates live under
  `tests/fixtures/verification/generated_candidates/`.

When a real bug is reproduced:

1. capture or author the repository-history fixture sequence that demonstrates
   the failure
2. add or accept a scenario that declares the behavioral claim and scenario
   classes involved
3. rerun the accepted scenario after the fix
4. keep the accepted scenario as a permanent regression entry in the
   maintained corpus

### Agent Playbook For Scenario and Bundle Generation

Use this decision rule when choosing what to create.

1. **Existing behavior already covered by an accepted scenario**
   - Reuse the accepted scenario in
     `tests/fixtures/verification/scenarios/`
   - Run it directly and inspect the resulting bundle
   - Do not create a new candidate unless the existing scenario is missing a
     materially different behavioral claim

2. **New feature behavior not yet covered**
   - Start from the feature specification
   - Generate a candidate into
     `tests/fixtures/verification/generated_candidates/`
   - Review it for:
     - stable behavioral claim
     - correct scenario class
     - durable assertions tied to public behavior
   - Promote it into `tests/fixtures/verification/scenarios/` only after
     review and successful execution

3. **Regression or bug-reproduction coverage**
   - Model the revision sequence in
     `tests/fixtures/verification/repos/`
   - Reference that history from an accepted scenario with
     `fixtures.repository_evolution`
   - Validate the scenario against the failing revision sequence and again
     after the fix
   - Retain the accepted regression scenario permanently

4. **CI/release gating**
   - Use only accepted scenarios from
     `tests/fixtures/verification/scenarios/`
   - Prefer corpus execution with `--accepted-dir ... --ci`
   - Use focused reruns with repeated `--scenario-id` values when triaging a
     specific regression

### Expected Bundle Shapes

- All runs should retain:
  - scenario definition
  - harness log
  - console output
  - CoreOps output
  - assertion results
- Failed runs should additionally retain failure-specific diagnostics
- Failed accepted regression scenarios should now also retain:
  - `failure-summary.txt`
  - `regression-summary.txt`
  - `promotion-status.txt`

### Scenario Authoring Conventions

Common scenarios should stay short and authorable.

- prefer named environment and policy profiles over repeating routine harness
  configuration inline
- use semantic CoreOps actions for common `apply`, `explain`, `plan`, and
  related steps
- use scenario-local overrides only when intentionally deviating from default
  profile behavior
- keep assertions tied to observable contract behavior rather than incidental
  implementation details

### Internal Synthetic Support

The hidden `--synthetic` switch exists only to support deterministic internal
tests and fast contract validation. It is not the intended product path and
should not be used for local signoff, CI gating, or release verification.

## Provenance Status Snapshot Workflow

- Canonical persisted provenance defaults to
  `/var/lib/core-ops/status.json`.
- `--state-file <path>` or `CORE_OPS_STATE_FILE` override that default when a
  different path is required.
- `core-ops status` reads the canonical snapshot directly and treats missing,
  partial, invalid, or unsupported snapshots as absent.
- Apply and agent flows update the canonical snapshot by default rather than
  maintaining a parallel persisted view.
- `core-ops apply --force-no-state` is an explicit escape hatch for running an
  apply without updating the canonical snapshot. It is intended for exceptional
  cases, not normal operation.
- Backward-incompatible persisted-schema changes require a recorded version
  review and a controller version update in `Cargo.toml` according to the
  project versioning policy.


## Deterministic Reconciliation Workflow

- Deterministic reconciliation uses three normalized inputs for a managed scope:
  desired, `last_applied`, and observed actual state.
- `core-ops plan` remains the review surface for this model. Repeated planning
  with identical normalized inputs must produce materially identical action,
  drift, and dependency ordering output.
- `core-ops apply` only advances `last_applied` after side effects complete and
  post-apply verification reports convergence. Partial, blocked, repeated-
  failure, and oscillating outcomes keep the last known-good revision intact.
- `core-ops apply --rollback-to <revision>` reuses the same planner and
  dependency ordering as forward reconciliation. Use
  `--rollback-plan-only` before execution when reviewing disruptive changes.
- Retry is bounded. Repeated failure or oscillation for the same affected object
  set stops automatic progress and records structured convergence diagnostics for
  operator review.

## Explainable Reconciliation Interface

- Machine-readable `plan`, `apply`, `result`, and `explain` outputs are the
  authoritative operator contract. Human-readable output must remain a
  deterministic rendering of those same view models.
- Human revision context keeps the immutable resolved revision primary and shows
  a meaningful requested ref secondarily, for example
  `454ac5f1 (demo-uat-v1)`.
- Persisted reconciliation and rollback semantics stay anchored to immutable
  revisions. Requested repository/ref values are operator-facing provenance
  only.
- Prior requested-ref context is only available after a successful apply has
  retained that revision with a build that knows how to store selector context.
- Default human output should stay concise:
  - `plan` emphasizes changed or recovery-relevant objects and collapses
    unchanged dependency trees unless they explain the outcome.
  - `apply` summaries contain only counts and overall outcome.
  - verbose `apply` may show translated phase progression and expanded
    diagnostics.
- `core-ops explain <object>` defaults to the currently deployed target from
  persisted state when `--repo` and `--rev` are omitted.

### Normalization Rules and Tolerated Runtime Variance

Supported managed resource kinds in this iteration are generated systemd units,
Quadlet resources, managed mounts, managed automounts, and rendered host
artifacts.

- Generated systemd units
  - Normalize by canonical unit name and stable field ordering.
  - Treat effective unit content, dependency directives, and enablement-relevant
    semantics as material.
  - Ignore formatting-only differences and transient runtime state such as the
    currently active PID.
- Quadlet resources
  - Normalize by canonical resource filename and generated unit identity.
  - Treat semantically relevant section keys and rendered content as material.
  - Ignore ordering and whitespace differences that do not change generated
    systemd behavior.
- Managed mounts
  - Normalize by native `.mount` unit identity derived from `Where=`.
  - Treat source, target path, filesystem type, mount options, and
    CoreOps-managed preparation semantics as material.
  - Ignore runtime-only counters or other non-semantic mount statistics.
- Managed automounts
  - Normalize by native `.automount` unit identity derived from `Where=`.
  - Treat the automount path and CoreOps-managed dependency semantics as
    material.
  - Ignore runtime-only activation timing details once the effective automount
    contract matches desired state.
- Rendered host artifacts
  - Normalize by canonical target path and stable content representation.
  - Treat rendered content and ownership/path semantics managed by CoreOps as
    material.
  - Ignore non-semantic formatting differences introduced during rendering.

When a difference is intentionally ignored, it must be explainable as
`runtime_variance` rather than being silently dropped from operator-visible
reasoning.

## Native Mount Management Workflow

- Author managed mounts as native `.mount` and optional `.automount` artifacts
  and embed only reconciliation-specific metadata in an `[X-CoreOps]` section.
- Express service-to-mount relationships in consuming unit content itself using
  native systemd directives such as `RequiresMountsFor=`, `After=`, and
  `Requires=` against managed `.mount` / `.automount` units.
- Keep ordinary `.mount` behavior as the default. Set `automount: true` only
  for explicitly network-backed mounts such as NFS.
- Keep `[X-CoreOps]` minimal in this iteration. `CreateMountpoint=true` is the
  default, and unsupported fields are rejected.
- `core-ops plan` should show native `.mount` stem references, dependency
  counts, and automount relationships when present.
- `core-ops apply` prepares bounded target paths, writes `.mount` and optional
  `.automount` units, and activates automount-backed mounts through the
  `.automount` unit instead of starting the `.mount` unit directly.
- Removing a managed mount stops dependent managed services first and fails
  explicitly if the mount is still busy or cannot be removed cleanly.

### Example Layout

```text
services/immich/
  immich.container
  var-lib-immich-media.mount
  var-lib-immich-media.automount
```

`services/immich/immich.container`

```ini
[Container]
Image=ghcr.io/immich-app/immich-server:release

[Service]
RequiresMountsFor=/var/lib/immich/media

[Unit]
After=var-lib-immich-media.automount var-lib-immich-media.mount
Requires=var-lib-immich-media.automount var-lib-immich-media.mount
```

`services/immich/var-lib-immich-media.mount`

```ini
[Unit]
After=network-online.target
Wants=network-online.target

[Mount]
What=nas:/volume1/media
Where=/var/lib/immich/media
Type=nfs
Options=rw,hard,noatime

[X-CoreOps]
CreateMountpoint=true
```

Optional `services/immich/var-lib-immich-media.automount`

```ini
[Automount]
Where=/var/lib/immich/media
```

### `[X-CoreOps]` Field Reference

- `CreateMountpoint`
  - Optional boolean.
  - Default: `true`.
  - Applies to the native `Where=` path from the `.mount` unit.
  - `true`: CoreOps creates the mountpoint directory if it is missing.
  - `false`: reconciliation fails if the mountpoint directory is missing.

### Override Rules

- Service-referenced managed mounts are keyed by native `.mount` stem.
- For those mounts, host overrides must not change the effective `Where=`
  value, because the stem is derived from that path.
- Host overrides may still change other native unit details such as `What=` or
  mount options, as long as the resulting layered unit remains valid.
- `[X-CoreOps]` follows the same layering order as native unit content. Later
  effective values override earlier ones before CoreOps validates the merged
  result.

### Troubleshooting

- `unsupported X-CoreOps field`
  - The `.mount` or `.automount` artifact still contains removed metadata.
  - Remove everything except `CreateMountpoint` from `[X-CoreOps]`.

- `mount unit name does not match Mount Where`
  - The `.mount` filename does not match the escaped systemd name derived from
    `Where=`.
  - Rename the file or fix `Where=` so they match.

- `automount unit name does not match Automount Where`
  - The `.automount` filename does not match the escaped systemd name derived
    from `Where=`.
  - Rename the file or fix `Where=` so they match.

- `mountpoint missing and CreateMountpoint=false`
  - CoreOps is configured not to create the mountpoint.
  - Provision the directory out of band, or set `CreateMountpoint=true`.

## VM Host Preparation (CoreOS)

### One-Time Preparation

```sh
sudo rpm-ostree override remove nfs-utils-coreos \
  --install nfs-utils \
  --install qemu-kvm \
  --install libvirt \
  --install virt-install
  
sudo systemctl restart

sudo systemctl enable --now libvirtd

coreos-installer download \
  --stream stable \
  --platform qemu \
  --format qcow2.xz
  
unxz fedora-coreos-*.qcow2.xz
mv fedora-coreos-*.qcow2 /var/lib/libvirt/images/fcos-base.qcow2

cat <<'EOF' | sudo tee /etc/polkit-1/rules.d/50-libvirt.rules
polkit.addRule(function(action, subject) {
  if (action.id == "org.libvirt.unix.manage" &&
      subject.user == "core") {
    return polkit.Result.YES;
  }
});
EOF

sudo nmcli connection add type bridge ifname br0
sudo nmcli connection add type bridge-slave ifname eth0 master br0
sudo nmcli connection modify bridge-br0 ipv4.method auto
sudo nmcli connection up bridge-br0

sudo mkdir -p /var/lib/libvirt/ignition
sudo chmod 0755 /var/lib/libvirt/ignition
sudo install -m 0644 minimal.ign /var/lib/libvirt/ignition/minimal.ign
```

### Preparing the VM

On the dev host:

```sh
just render-ignition minimal
scp infra/ignition/minimal.ign core@$VM_HOST:/var/lib/libvirt/images/
```

The feature-008 verification harness treats this `just render-ignition` plus
`infra/ignition` flow as the initial provisioning substrate for disposable
guest bootstrapping. If the harness later adopts a cleaner provisioning
abstraction, manual UAT should migrate onto that path rather than maintaining a
parallel setup workflow.

On the VM host:

```
sudo mkdir -p /var/lib/libvirt/ignition
sudo chmod 0755 /var/lib/libvirt/ignition
sudo install -m 0644 minimal.ign /var/lib/libvirt/ignition/minimal.ign
```

On the dev host:

```sh
virsh -c qemu+ssh://core@$VM_HOST/system pool-define-as default dir --target /var/lib/libvirt/images
virsh -c qemu+ssh://core@$VM_HOST/system pool-start default
virsh -c qemu+ssh://core@$VM_HOST/system pool-autostart default

virsh -c qemu+ssh://core@$VM_HOST/system pool-list --all

virsh -c qemu+ssh://core@$VM_HOST/system vol-create-as default core-ops-uat.qcow2 10G \
  --format qcow2 \
  --backing-vol /var/lib/libvirt/images/fcos-base.qcow2 \
  --backing-vol-format qcow2

virt-install \
  --connect qemu+ssh://core@$VM_HOST/system \
  --name core-ops-uat \
  --osinfo fedora-coreos-stable \
  --memory 4096 \
  --vcpus 2 \
  --import \
  --disk vol=default/core-ops-uat.qcow2,format=qcow2 \
  --network bridge=br0,model=virtio \
  --graphics none \
  --noautoconsole \
  --qemu-commandline="-fw_cfg name=opt/com.coreos/config,file=/var/lib/libvirt/ignition/minimal.ign"
```

### Tearing Down the VM

```sh
virsh -c qemu+ssh://core@$VM_HOST/system destroy core-ops-uat
virsh -c qemu+ssh://core@$VM_HOST/system undefine core-ops-uat
virsh -c qemu+ssh://core@$VM_HOST/system vol-delete --pool default core-ops-uat
```

### UAT VM Reset

```sh
virsh -c qemu+ssh://core@$VM_HOST/system destroy core-ops-uat
sudo rm /var/lib/libvirt/images/core-ops-uat.qcow2
sudo qemu-img create -f qcow2 \
  -b /var/lib/libvirt/images/fcos-base.qcow2 \
  /var/lib/libvirt/images/core-ops-uat.qcow2
virsh -c qemu+ssh://core@$VM_HOST/system start core-ops-uat
```
