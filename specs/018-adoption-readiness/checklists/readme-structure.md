# README Structural Checklist (spec/018)

**Purpose**: Runnable acceptance-criteria runbook for `README.md` after the spec/018 restructure. Used at PR review time and whenever a future contributor proposes a README edit.

## How to use

1. Run each command from the repository root.
2. Compare the actual output against the expected outcome.
3. Any failure indicates a deviation from the structural contract; resolve before merge.

The SC-006a sanitization grep (C-006 below) is the canonical home of the operator-private stop-list pattern. The pattern is intentionally **not** embedded in `docs/onboarding-script.sh` because the regex would self-match against its own literal occurrence in the script.

## Checks

### C-001 — README line budget (FR-003 / SC-001)

```sh
wc -l README.md
```

Expected: ≤ 400. If exceeded, compress philosophy sections (Why CoreOps exists, What CoreOps is not, AI authorship) before extending — the budget is a deliberate constraint on README scope, not a soft target.

### C-002 — Badge row composition (FR-002 / SC-002)

```sh
sed -n '1,/^---$/p' README.md | grep -cE '<a href=.*<img src='
```

Expected: `4` — exactly four `<a href="..."><img src="..." alt="..."></a>` badges, in the order CI / E2E Gate / Latest Release / License, between the title block and the first `---` separator. No additional badges may be promoted to the top row in this slice.

For the alt-text ordering check:

```sh
sed -n '1,/^---$/p' README.md | grep -oE 'alt="[^"]+"' | sed 's/alt="//; s/"$//'
```

Expected output (in order):

```
CoreOps logo
CI
E2E Gate
Latest Release
License: AGPL-3.0-or-later
```

### C-003 — Mental-model heading position (FR-001 §3 / SC-003)

```sh
grep -n '^## 30-second mental model' README.md | head -1
```

Expected: exactly one match, line number ≤ 120. If the heading drifted past line 120, a section was inserted above it that doesn't belong above the mental model in the FR-001 ordering.

### C-004 — Mermaid block presence and required substrings (FR-004 / SC-004)

```sh
grep -c '^```mermaid' README.md
awk '/^```mermaid/,/^```$/' README.md | grep -cE 'Git|core-ops|systemd'
awk '/^```mermaid/,/^```$/' README.md | grep -cE '\-\.->'
```

Expected: first command ≥ 1; second command ≥ 3 (each substring `Git`, `core-ops`, `systemd` appears at least once; counts may sum higher); third command ≥ 1 (audit/status side outputs use dashed edges).

### C-005 — Walkthrough two-block + line budget (FR-006 / SC-007, SC-007b)

```sh
awk '/^## What using CoreOps feels like/{f=1; next} /^## Real-world examples/{f=0} f' README.md > /tmp/walkthrough.txt

grep -c '^```text' /tmp/walkthrough.txt

awk '/^```text/{f=1; next} /^```$/{f=0; next} f' /tmp/walkthrough.txt | grep -cv '^[[:space:]]*$'

grep -oE 'immich-database|immich-server|immich-redis|immich-ml|traefik-edge|immich-internal|immich-public|immich-db-data|immich-ml-cache|immich-upload' /tmp/walkthrough.txt | sort -u | wc -l
```

Expected: first command = `2` (exactly two fenced text blocks); second command ≤ ~25 (combined non-blank lines across both blocks); third command ≥ 1 (at least one recognizable Quadlet unit identifier from `examples/03-immich/services/`).

### C-006 — Sanitization stop-list (FR-009a / SC-006a)

```sh
grep -iE '(not\.one|ulthar|192\.168\.|10\.0\.|172\.16\.)' docs/onboarding.cast docs/onboarding-script.sh
```

Expected: zero matches (exit 1 from `grep`). If matches surface, the cast or script leaks operator-private values; re-record with a cleaner shell environment, or extend the post-recording strip pass (see "OSC 3008 sanitization note" below).

**OSC 3008 sanitization note**: `pam_systemd` may emit OSC 3008 session-tracking sequences (containing `hostname=`, `machineid=`, PID metadata) when `sudo` creates a session under a terminal that supports them. The SC-006a regex above does not catch these sequences, but FR-009a's broader sanitization rule prohibits the leaks they contain. The recording committed at spec/018 was post-processed to strip OSC 3008 before commit; if regenerating, repeat the strip pass:

```sh
python3 - <<'PY'
import json, re
osc3008 = re.compile(r'\x1b\]3008;.*?\x1b\\', re.DOTALL)
with open("docs/onboarding.cast") as f: lines = f.readlines()
hdr = json.loads(lines[0])
if "env" in hdr and "/nix/store/" in hdr["env"].get("SHELL", ""):
    hdr["env"]["SHELL"] = "/bin/bash"
if len(lines) > 1:
    hdr["duration"] = json.loads(lines[-1])[0]
out = [json.dumps(hdr) + "\n"]
for line in lines[1:]:
    if not line.strip(): continue
    ev = json.loads(line)
    if len(ev) >= 3 and isinstance(ev[2], str):
        ev[2] = osc3008.sub("", ev[2])
    out.append(json.dumps(ev) + "\n")
with open("docs/onboarding.cast", "w") as f: f.writelines(out)
PY
```

### C-007 — Hype stop-list (FR-014 / SC-008)

```sh
grep -iE '(enterprise-ready|industry-leading|production-grade|🚀)' README.md
```

Expected: zero matches.

### C-008 — Pre-018 link targets resolve (Compatibility / SC-009)

```sh
for target in LICENSE CHANGELOG.md CODE_OF_CONDUCT.md docs/development.md examples/01-caddy-whoami examples/02-nextcloud examples/03-immich examples/04-traefik-authelia examples/05-observability; do
  test -e "$target" || echo "MISSING: $target"
done
```

Expected: no `MISSING:` lines printed.

Catch-all version (extracts every `[label](path)` link target from README and tests existence; skips http/https links and anchor-only fragments):

```sh
grep -oE '\]\(([^)#)]+)\)' README.md \
  | sed -E 's/\]\(|\)//g' \
  | grep -vE '^https?://|^#' \
  | while read -r target; do
      test -e "$target" || echo "MISSING: $target"
    done
```

Expected: no `MISSING:` lines printed.

### C-009 — No third-party JS embed (FR-013)

```sh
grep -E '(asciinema\.org/.*\.js|<iframe|<script)' README.md
```

Expected: zero matches. The asciicast is referenced as the in-tree `docs/onboarding.cast` link only; a clickable text link to an asciinema.org-hosted upload is permitted, but no `<script>` or `<iframe>` embed.

### C-010 — Asciicast format and duration (FR-007 / FR-008 / SC-005, SC-005a)

```sh
test -f docs/onboarding.cast
head -n 1 docs/onboarding.cast | jq '.version'
head -n 1 docs/onboarding.cast | jq '.duration'
```

Expected: file exists; `.version` is `2`; `.duration` is a number ≤ `90`. If `.duration` is `null`, asciinema 2.4.0 omitted it on completion — re-run the post-processing block from C-006 above (it computes duration from the last event timestamp).

### C-011a — GIF sidecar exists and is embedded inline (FR-007 / FR-013 / SC-005b)

```sh
test -f docs/assets/core-ops-demo.gif
head -c 6 docs/assets/core-ops-demo.gif | grep -E '^GIF8[79]a'
wc -c < docs/assets/core-ops-demo.gif
grep -E 'docs/assets/core-ops-demo\.gif' README.md
```

Expected: file exists; first 6 bytes are `GIF87a` or `GIF89a`; size ≤ 1 048 576 bytes (≤ 1 MB soft cap); the README references the file via image syntax inside the `## What using CoreOps feels like` section.

### C-011 — Onboarding regeneration script invariants (FR-009 / SC-006)

```sh
test -x docs/onboarding-script.sh
head -1 docs/onboarding-script.sh
grep -c 'examples/03-immich' docs/onboarding-script.sh
```

Expected: file is executable; first line is `#!/usr/bin/env bash`; literal `examples/03-immich` appears at least once.

---

See `specs/018-adoption-readiness/spec.md` for the FR/SC definitions this checklist implements. The corresponding tasks are C-001/C-007 → T021/T022; C-005 → T013/T023; C-006/C-010/C-011 → T015/T014/T003; C-008 → T023; C-009 → T013/T016 (FR-013 carrying through to anything that touches the walkthrough section).
