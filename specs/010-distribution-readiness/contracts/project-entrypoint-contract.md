# Contract: Project Entrypoint

## Purpose

Defines the minimum stable public content required at the project entrypoint
for outside consumption.

## Required Sections

1. Project framing statement
2. Goals
3. Non-goals
4. Target audience
5. Supported system classes
6. Unsupported usage modes
7. AI authorship disclosure and guarantee explanation
8. Minimal trust story
9. Credibility surface
10. Installation path link
11. Operator verification flow link
12. License link
13. Code of conduct link
14. Logo or reserved logo placeholder

## Credibility Surface Rules

The entrypoint MUST expose a compact credibility surface containing:

- latest release identity
- current release-gate result
- accepted verification result
- currently available published binary artifacts

The credibility surface MUST:

- appear in the project entrypoint
- use stable placement across releases
- keep the required values consistently locatable across releases
- refer to the same release identity and gate state as the underlying release
  materials

## Support Boundary Rules

The entrypoint MUST state:

- Fedora CoreOS is the officially tested and supported environment
- other systemd-based hosts are theoretically compatible but untested
- non-systemd environments are unsupported
- running CoreOps from a container is not a supported consumption method

## Validation Questions

- Can a first-time evaluator determine fit, limits, and trust within 10
  minutes?
- Can the same public credibility values be found in the same place after a
  new release?
- Are unsupported environments and modes rejected explicitly rather than
  implied?
