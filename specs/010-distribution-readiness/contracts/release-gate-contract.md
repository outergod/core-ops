# Contract: Release Gate And Verification Environment

## Purpose

Defines the public decision contract for treating a build as
distribution-ready.

## Required Gate Inputs

- build result
- accepted verification result
- scenario/schema or spec conformance result
- determinism result
- authoritative verification environment identity

## Ready Decision

A release candidate may be treated as distribution-ready only when all required
gate inputs pass.

## Failure Decision

The release gate MUST fail closed when any required input:

- fails
- is missing
- cannot be attributed to the current release identity
- cannot be attributed to a documented authoritative verification environment

## Verification Environment Identity

The authoritative verification environment MUST be:

- documented
- reproducible
- versioned sufficiently to detect drift
- attributable to release-gate output and maintained materials

## Idempotence Rules

- Re-running the same gate under materially unchanged inputs MUST yield the
  same decision.
- Re-running the same gate in the same documented verification-environment
  definition MUST not silently change environment identity.

## Validation Questions

- Can an operator tell which environment was used for the gate?
- Can maintainer drift in a self-hosted runner be detected from maintained
  materials?
- Does the release gate prevent publication when public CI checks, protected
  authoritative E2E checks, conformance checks, or determinism checks fail?
