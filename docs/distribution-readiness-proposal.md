# CoreOps Distribution Readiness

At this point, CoreOps should be put into shape for first attempts of outside consumption. 

A system is distribution-ready when a competent stranger can install it, run it, verify its behavior, and understand its limits—without talking to you.

## Requirements

### README.md

A README that clearly expresses the goals and non-goals of the project.

*Framing*: _A declarative convergence engine for systemd-based hosts_.
*Goal*: Making CoreOS more useful by filling the gap between ignition-based configuration (downtime, user-unfriendly, more useful for k8s nodes) and Ansible/Puppet (imperative, bad match for the architecture). systemd native, Quadlet support.
*Non-Goals*:
- Container orchestration (Kubernetes)
- Ansible (generic configuration management)
- Templating / DSL
- Imperative orchestration
*Target Audience*: Homelabbers, SME infra teams

Also:
- Examples of what's possible today
- Examples what's not possible (yet)
- Theory of the system to show where future releases are going
- AI disclaimer (everything made by AI, but spec-driven)
- Logo

### CI

Integrate with GitHub Actions. Build continuously. Release-gate.

Minimum bar:
- Build succeeds
- E2E harness runs on real CoreOS (via self-hosted runner)
- All accepted scenarios pass

Spec conformance checks
- Are scenarios valid against the spec schema?

Determinism checks
- Same scenario → same result (no hidden nondeterminism)

Version stamping
Every build tied to:
- git commit
- spec version
- binary version

### CD

- Raw binary download as the supported first distribution form
- Release-gated publication of binary archives plus checksums and license metadata

### Installation Story

How does CoreOps enter the system?
- documented binary acquisition
- explicit install step
- first command
- smoke test
- minimal operator verification flow

### Versioning

Make sure the version is visible in
- CLI
- logs
- explain output (ideally)
- verification and release materials

### Changelog

keepachangelog format. Backfill and anchor with constitution.

### Failure ergonomics

What happens when things go wrong?

- meaningful exit codes
- actionable error messages
- logs that can be reasoned about

### Minimal trust story

- what does it modify?
- how reversible is it?
- how does a user audit what happened?

### Support Boundary

- Officially supported and tested: Fedora CoreOS
- Theoretically compatible but untested: other systemd-based hosts
- Unsupported: non-systemd environments
- Running CoreOps from a container is not a supported consumption method
