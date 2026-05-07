#!/usr/bin/env bash
# docs/onboarding-script.sh — regeneration entry point for docs/onboarding.cast
#
# Records an asciinema session of `core-ops apply --source-repo
# examples/03-immich --host example` — the single beat where the
# spec/006 streaming output is operationally interesting (the
# line-by-line "creating... → created" progression as podman/systemd
# processes each unit). Static plan and idempotent-re-run blocks
# live in the README walkthrough section (FR-006); they do not
# need motion. The cast and its rendered GIF sidecar at
# `docs/assets/core-ops-demo.gif` (produced by `agg`) are linked
# from the README walkthrough section per spec/018 FR-005 / FR-007 /
# FR-009.
#
# ── Versions (asserted at re-record time) ────────────────────────────
#   asciinema 2.4.0    (nix devshell pin; see flake.nix)
#   asciicast format   v2 (FR-008 / SC-005)
#   agg 1.7.0          (asciinema-agg; nix devshell pin) — renders the
#                      cast to docs/assets/core-ops-demo.gif (FR-007 /
#                      SC-005b). The GIF is the inline-renderable
#                      sidecar the README embeds.
#
# ── Operator prerequisites ───────────────────────────────────────────
#   * `nix develop` shell (provides asciinema 2.4.0).
#   * `core-ops` binary installed at /usr/local/bin/core-ops. The
#     env-scrubbed PATH below is `/usr/local/bin:/usr/bin:/bin`; the
#     binary must be on that PATH or the recorded subshell will not
#     find it. Build with `cargo build --release` then
#     `install -m 0755 target/release/core-ops /usr/local/bin/core-ops`,
#     or unpack a published release bundle as documented in README
#     ## Quick start.
#   * Working tree at the repo root (the script `cd`s into the repo so
#     the recorded commands can reference `examples/03-immich` as a
#     project-relative path).
#   * `examples/03-immich/hosts/example/` overlay matches the recording
#     host's device shape, OR the GPU device path declared in
#     `examples/03-immich/services/immich-ml/quadlet/immich-ml.container.d/20-gpu.conf`
#     is replaced with a placeholder, OR `immich-ml` is temporarily
#     dropped from `hosts/example/host.yaml` for the recording. (Spec/018
#     edge case: GPU passthrough cannot be exercised on an arbitrary host.)
#
# ── Sanitization (FR-009a, mirrors spec/017 FR-009) ──────────────────
#   * Prompt: `op@example $` (no operator hostname).
#   * Paths: project-relative (`examples/03-immich`) or `/home/op/...`.
#   * Domains: RFC 2606 reserved (`example`, `op`, `host`, `*.example`).
#   * IPs: RFC 5737 documentation ranges only.
#   * No operator-private hostnames, no real credentials, no
#     environment values sourced from the operator's private setup.
#   The env-scrubbed `bash --noprofile --norc` subshell below ensures
#   `$HOSTNAME`, `$USER`, shell history, and operator dotfiles cannot
#   leak into the recording.
#
# ── Regeneration ─────────────────────────────────────────────────────
#   nix develop --command docs/onboarding-script.sh
#
# ── Post-recording verification ──────────────────────────────────────
#   head -n 1 docs/onboarding.cast | jq '.version'    # → 2  (SC-005)
#   head -n 1 docs/onboarding.cast | jq '.duration'   # ≤ 90 (SC-005a)
#   asciinema play docs/onboarding.cast               # plays end-to-end
#   The SC-006a stop-list grep (operator-private hostnames + RFC 1918
#   ranges) is documented in the structural checklist at
#   `specs/018-adoption-readiness/checklists/readme-structure.md`. Run
#   it from there — keeping the regex out of this script avoids a
#   self-match against the very pattern the check forbids.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

OUTPUT="docs/onboarding.cast"
GIF_OUTPUT="docs/assets/core-ops-demo.gif"
ASCIINEMA_PINNED_VERSION="2.4.0"
AGG_PINNED_VERSION="1.7.0"

if ! command -v asciinema >/dev/null 2>&1; then
  echo "error: asciinema not on PATH; run inside 'nix develop' shell" >&2
  exit 1
fi

if ! command -v agg >/dev/null 2>&1; then
  echo "error: agg not on PATH; run inside 'nix develop' shell" >&2
  exit 1
fi

actual_version="$(asciinema --version 2>/dev/null | awk '{print $2}')"
if [[ "$actual_version" != "$ASCIINEMA_PINNED_VERSION" ]]; then
  echo "warning: asciinema $actual_version detected; spec/018 pin is $ASCIINEMA_PINNED_VERSION" >&2
fi

actual_agg_version="$(agg --version 2>/dev/null | awk '{print $2}')"
if [[ "$actual_agg_version" != "$AGG_PINNED_VERSION" ]]; then
  echo "warning: agg $actual_agg_version detected; spec/018 pin is $AGG_PINNED_VERSION" >&2
fi

mkdir -p "$(dirname "$GIF_OUTPUT")"

# Recorded command sequence — written to a temp file so we avoid the
# nested-quoting trap when passing it through `asciinema --command` and
# `env -i ... bash -c`. `demo` echoes the prompt + command before
# executing so the cast playback shows operator-style cadence without
# requiring a real interactive shell.
RECORDED_SCRIPT="$(mktemp --tmpdir onboarding.XXXXXX.sh)"
trap 'rm -f "$RECORDED_SCRIPT"' EXIT

cat > "$RECORDED_SCRIPT" <<'BASH'
demo() {
  printf 'op@example $ %s\n' "$1"
  eval "$1"
  printf '\n'
}

# Single beat — `apply` is the operationally interesting moment: it
# streams the line-by-line "creating... → created" progression in
# real time as podman/systemd processes each unit. The static
# `plan` and idempotent re-run code blocks live in the README's
# walkthrough section (per FR-006) and do not benefit from
# motion; including them in the cast wastes the 90 s budget on
# uneventful output.
demo 'sudo core-ops apply --source-repo examples/03-immich --host example'
BASH

# `--idle-time-limit=2` compresses any pause ≥ 2s in playback so the
# 90s duration cap (FR-007 / SC-005a) is not eaten by I/O stalls.
asciinema rec \
  --overwrite \
  --idle-time-limit=2 \
  --rows=30 --cols=110 \
  --command "env -i HOME=/home/op PATH=/usr/local/bin:/usr/bin:/bin TERM=xterm-256color PS1='op@example \$ ' bash --noprofile --norc '$RECORDED_SCRIPT'" \
  "$OUTPUT"

echo
echo "Recorded: $OUTPUT"
echo "Duration (must be ≤ 90):"
head -n 1 "$OUTPUT" | jq '.duration'

# Render the GIF sidecar that the README embeds inline (FR-007 / SC-005b).
# `--idle-time-limit 2` mirrors the recording-side compression so the
# rendered animation matches the cast's playback cadence.
agg --idle-time-limit 2 --quiet "$OUTPUT" "$GIF_OUTPUT"
echo "Rendered: $GIF_OUTPUT"
echo "Size: $(wc -c < "$GIF_OUTPUT") bytes (SC-005b soft cap: 1 MB)"

echo
echo "Next: run the SC-006a sanitization stop-list grep from"
echo "  specs/018-adoption-readiness/checklists/readme-structure.md"
echo "against $OUTPUT and this script."
