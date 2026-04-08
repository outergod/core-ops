# Contract: Operator Install And Verify Flow

## Purpose

Defines the supported outside-consumption flow for obtaining CoreOps, running
it for the first time, and verifying correct behavior on a supported system.

## Installation Path Requirements

A supported installation path MUST document:

1. how to obtain the artifact
2. how to install it
3. how to run the first command
4. how to validate success through a smoke test

The installation path MUST:

- be deterministic
- minimize external dependencies where feasible
- avoid hidden prerequisites or undocumented operator knowledge
- succeed on a freshly provisioned supported environment

## Verification Flow Requirements

The minimal operator-facing verification flow MUST:

- run on the user’s own system
- not require internal project infrastructure
- include at least one observable state change or convergence check
- define the expected outcome explicitly
- be reproducible on the same supported system class

## Supported System Rules

- Officially supported: Fedora CoreOS
- Theoretically compatible but untested: other systemd-based hosts
- Unsupported: non-systemd environments
- Unsupported consumption mode: running CoreOps from a container

## Validation Questions

- Can a competent stranger complete the documented install and verify flow on a
  fresh Fedora CoreOS host without maintainer help?
- Is the verification step based on a reproducible observable outcome rather
  than an informal “looks good” check?
- Are unsupported environments or execution modes clearly excluded?
