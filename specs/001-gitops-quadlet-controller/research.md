# Research: GitOps Quadlet Controller

**Date**: 2026-03-18

## Decisions

### Quadlet file locations and systemd integration

- **Decision**: Manage desired workloads by writing Quadlet files into the
  system-level Quadlet search path and triggering `systemctl daemon-reload`, then
  manage the generated units with standard systemd commands.
- **Rationale**: Quadlet files are read from `/etc/containers/systemd/` and other
  system paths and are converted into systemd units at reload time; the resulting
  units are managed like any other systemd service. This aligns with native
  system primitives and avoids custom orchestration logic.
- **Alternatives considered**: Using `podman generate systemd` (deprecated) or
  managing containers directly without Quadlet. Quadlet is the recommended path.
- **Sources**:
  - https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html
  - https://docs.podman.io/en/v4.7.2/markdown/podman-systemd.unit.5.html
  - https://docs.podman.io/en/latest/markdown/podman-generate-systemd.1.html

### Fedora CoreOS configuration and immutable model

- **Decision**: Treat Fedora CoreOS as an image-based system configured via
  Ignition at first boot; do not attempt base OS mutation from the controller.
- **Rationale**: Fedora CoreOS instances are customized on first boot via
  Ignition, which applies configuration during initramfs, and system updates are
  image/ostree-based rather than ad-hoc package mutation. This supports strict
  mutation boundaries for the controller.
- **Alternatives considered**: Host-level configuration management or package
  layering from the controller. These are out of scope for MVP.
- **Sources**:
  - https://docs.stg.fedoraproject.org/en-US/fedora-coreos/getting-started/
  - https://github.com/coreos/ignition
  - https://github.com/coreos/rpm-ostree

### Implementation language and runtime form

- **Decision**: Use Rust (stable toolchain) to build a CLI and a long-running
  controller service.
- **Rationale**: Rust provides strong safety guarantees for host-level tooling and
  produces a single static binary suitable for CoreOS environments.
- **Alternatives considered**: Go (simplicity and fast iteration), Python (faster
  prototyping). Rejected due to weaker safety guarantees or heavier runtime.

### Storage and state

- **Decision**: Use filesystem storage only for MVP (Quadlet files, generated
  unit references, reconciliation state cache, and audit logs).
- **Rationale**: The controller is single-host and does not require external
  databases. Files are durable and align with immutable OS constraints.
- **Alternatives considered**: Embedded databases or external storage. Not needed
  for MVP scope.

### Testing approach

- **Decision**: Use unit tests for pure planning/state logic and integration
  tests that exercise Quadlet/systemd behavior in a controlled environment.
- **Rationale**: Invariants and convergence must be validated at the behavioral
  level while keeping pure logic testable and deterministic.
- **Alternatives considered**: Only unit tests (insufficient for host-level
  behavior) or only end-to-end tests (too slow and opaque).
