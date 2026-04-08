# Quickstart: Distribution Readiness

## Goal

Demonstrate the intended outside-consumption path for a first-time operator on
a freshly provisioned Fedora CoreOS host using the published binary
distribution.

## Preconditions

- Fresh Fedora CoreOS system
- Network access sufficient to download the published CoreOps binary
- No undeclared host preparation beyond what the installation instructions
  themselves document

## Steps

1. Open the project entrypoint and confirm:
   - project framing and non-goals
   - supported and unsupported system classes
   - license and code of conduct
   - credibility surface with latest release, release-gate result, accepted
     verification result, and available binary artifacts
2. Download the current published CoreOps binary artifact from the documented
   binary distribution surface, including the canonical `core-ops.service` and
   `core-ops.timer` unit files.
3. Install the binary and canonical unit files using the documented
   direct-consumption instructions.
4. Run the documented first command and confirm the binary reports visible
   version identity.
5. Configure or inspect the canonical `core-ops.service` and
   `core-ops.timer` unattended execution path.
6. Execute the documented minimal operator verification flow.
7. Confirm the verification flow yields the explicitly documented expected
   outcome on the fresh Fedora CoreOS system.
8. Inspect the documented diagnostic or provenance surface and confirm the same
   release identity is visible there.

## Expected Results

- A first-time operator can complete the flow without maintainer assistance.
- The install path does not rely on hidden prerequisites.
- The supported service/timer integration path is discoverable and matches the
  host-native unattended execution model.
- The verification flow produces a reproducible observable success condition.
- Version identity is visible and consistent across declared surfaces.
- The release materials match what the project entrypoint claims.

## Failure Checks

- Missing binary artifact or mismatched artifact link
- Missing canonical `core-ops.service` or `core-ops.timer` release artifact
- Missing first-command or smoke-test instructions
- Verification flow lacks an explicitly defined expected outcome
- Release identity differs between CLI, diagnostics, and release materials
- Entry point omits license, code of conduct, or support-boundary statements
