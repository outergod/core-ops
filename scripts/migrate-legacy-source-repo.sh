#!/usr/bin/env bash
# migrate-legacy-source-repo.sh — convert a CoreOps source repository
# from the pre-spec-016 layout to the formalized layout in one
# mechanical pass.
#
# Per specs/016-source-repository-layout/research.md D10 the migration
# is file-moves only, idempotent, and produces a destination set
# identical to the pre-migration plan (SC-003).
#
# Usage:
#     scripts/migrate-legacy-source-repo.sh <path-to-source-repo>
#
# The script exits non-zero with a clear message on any unrecognized
# layout shape; it MUST NOT corrupt a partially-migrated tree.

set -euo pipefail

if [[ $# -ne 1 ]]; then
    printf 'usage: %s <path-to-source-repo>\n' "$0" >&2
    exit 64
fi

REPO=$1
if [[ ! -d "$REPO" ]]; then
    printf 'error: %s is not a directory\n' "$REPO" >&2
    exit 66
fi
if [[ ! -d "$REPO/services" || ! -d "$REPO/hosts" ]]; then
    printf 'error: %s is not a CoreOps source repository (missing services/ or hosts/)\n' "$REPO" >&2
    exit 66
fi

# Allowed extensions per payload kind, used to recognize unit files
# without invoking the binary.
quadlet_exts=("container" "volume" "network" "pod")
systemd_exts=("socket" "timer" "target" "mount" "path" "automount")

is_quadlet_ext() {
    local ext=$1
    local candidate
    for candidate in "${quadlet_exts[@]}"; do
        [[ "$ext" == "$candidate" ]] && return 0
    done
    return 1
}

is_systemd_ext() {
    local ext=$1
    local candidate
    for candidate in "${systemd_exts[@]}"; do
        [[ "$ext" == "$candidate" ]] && return 0
    done
    return 1
}

# unit_owner <unit-with-extension>
# Resolves which service id owns <unit>.<ext> by scanning every
# services/<svc>/quadlet/ and services/<svc>/systemd/ directory.
# Prints the owning service id on stdout. Exits 1 (without printing)
# if there is no owner; exits 2 if there are multiple owners.
unit_owner() {
    local unit=$1
    local owners=()
    local svc_dir
    for svc_dir in "$REPO/services"/*/; do
        [[ -d "$svc_dir" ]] || continue
        local svc
        svc=$(basename "$svc_dir")
        if [[ -e "$svc_dir/quadlet/$unit" ]] || [[ -e "$svc_dir/systemd/$unit" ]]; then
            owners+=("$svc")
        fi
    done
    case "${#owners[@]}" in
        0) return 1 ;;
        1) printf '%s\n' "${owners[0]}"; return 0 ;;
        *)
            printf 'error: host drop-in unit %s is owned by multiple services: %s\n' \
                "$unit" "${owners[*]}" >&2
            printf 'rename the unit or split the drop-in by service before re-running migration\n' >&2
            return 2
            ;;
    esac
}

# Phase 1 — services.
# For each services/<svc>/:
#   1.a Reassign quadlet/<unit>.<systemd-ext> to systemd/<unit>.<systemd-ext>.
#   1.b Move quadlet-overrides/<unit>.<ext>.d/<file> to quadlet/<unit>.<ext>.d/<file>.
#   1.c Flatten config/etc/<root>/* into config/* (synthesizing
#       service.yaml when <root> != <svc-id>).
for svc_dir in "$REPO/services"/*/; do
    [[ -d "$svc_dir" ]] || continue
    svc=$(basename "$svc_dir")

    # 1.a quadlet/<unit>.<systemd-ext> → systemd/<unit>.<systemd-ext>
    if [[ -d "$svc_dir/quadlet" ]]; then
        for unit_path in "$svc_dir/quadlet"/*; do
            [[ -f "$unit_path" ]] || continue
            unit_name=$(basename "$unit_path")
            ext="${unit_name##*.}"
            if is_systemd_ext "$ext"; then
                mkdir -p "$svc_dir/systemd"
                mv -- "$unit_path" "$svc_dir/systemd/$unit_name"
            elif ! is_quadlet_ext "$ext"; then
                printf 'error: unrecognized unit extension %s in %s\n' "$ext" "$unit_path" >&2
                exit 65
            fi
        done
    fi

    # 1.b quadlet-overrides/<unit>.<ext>.d/* → quadlet/<unit>.<ext>.d/*
    if [[ -d "$svc_dir/quadlet-overrides" ]]; then
        for dropin_dir in "$svc_dir/quadlet-overrides"/*.d; do
            [[ -d "$dropin_dir" ]] || continue
            dropin_name=$(basename "$dropin_dir")
            mkdir -p "$svc_dir/quadlet/$dropin_name"
            for conf in "$dropin_dir"/*; do
                [[ -f "$conf" ]] || continue
                mv -- "$conf" "$svc_dir/quadlet/$dropin_name/$(basename "$conf")"
            done
            rmdir -- "$dropin_dir"
        done
        rmdir -- "$svc_dir/quadlet-overrides" 2>/dev/null || true
    fi

    # 1.c config/etc/<root>/* → config/*  (+ generate service.yaml if root != svc)
    if [[ -d "$svc_dir/config/etc" ]]; then
        # Expect exactly one <root> directory under config/etc/ in a
        # well-formed legacy tree. Scanning all entries is more robust
        # against hand-edited variants and surfaces malformed cases as
        # explicit errors.
        for root_dir in "$svc_dir/config/etc"/*/; do
            [[ -d "$root_dir" ]] || continue
            config_root=$(basename "$root_dir")
            # Move every file under <root>/ to config/<rel>.
            while IFS= read -r -d '' file; do
                rel=${file#"$root_dir"}
                target="$svc_dir/config/$rel"
                mkdir -p "$(dirname "$target")"
                mv -- "$file" "$target"
            done < <(find "$root_dir" -type f -print0)
            # Remove now-empty subdirectories rooted at <root>/.
            find "$root_dir" -type d -empty -delete 2>/dev/null || true

            # Variant: synthesize service.yaml if the legacy config-root
            # differs from the service id.
            if [[ "$config_root" != "$svc" ]]; then
                manifest="$svc_dir/service.yaml"
                if [[ -f "$manifest" ]]; then
                    # Already migrated or hand-authored; sanity-check the value.
                    if ! grep -q "^config-root: ${config_root}\b" "$manifest"; then
                        printf 'error: %s exists but does not declare config-root: %s\n' \
                            "$manifest" "$config_root" >&2
                        exit 65
                    fi
                else
                    printf 'config-root: %s\n' "$config_root" > "$manifest"
                fi
            fi
        done
        # Remove the empty config/etc/ scaffold.
        find "$svc_dir/config/etc" -type d -empty -delete 2>/dev/null || true
    fi
done

# Phase 2 — hosts.
# For each hosts/<h>/overrides/:
#   2.a quadlet/<unit>.<ext>.d/<file> → <svc-id>/quadlet/<unit>.<ext>.d/<file>
#   2.b config/etc/<root>/<rel> → <svc-id>/config/<rel>
# <svc-id> is resolved through unit_owner (drop-ins) or the matching
# service whose config-root is <root> (config files).
for host_dir in "$REPO/hosts"/*/; do
    [[ -d "$host_dir" ]] || continue
    overrides="$host_dir/overrides"
    [[ -d "$overrides" ]] || continue

    # 2.a host quadlet drop-ins
    if [[ -d "$overrides/quadlet" ]]; then
        for dropin_dir in "$overrides/quadlet"/*.d; do
            [[ -d "$dropin_dir" ]] || continue
            dropin_name=$(basename "$dropin_dir")
            unit="${dropin_name%.d}"
            owner=$(unit_owner "$unit") || {
                rc=$?
                if [[ $rc -eq 1 ]]; then
                    printf 'error: host drop-in %s references unit %s which is not owned by any service\n' \
                        "$dropin_dir" "$unit" >&2
                fi
                exit 65
            }
            target_dir="$host_dir/$owner/quadlet/$dropin_name"
            mkdir -p "$target_dir"
            for conf in "$dropin_dir"/*; do
                [[ -f "$conf" ]] || continue
                mv -- "$conf" "$target_dir/$(basename "$conf")"
            done
            rmdir -- "$dropin_dir"
        done
        rmdir -- "$overrides/quadlet" 2>/dev/null || true
    fi

    # 2.b host config overrides
    if [[ -d "$overrides/config/etc" ]]; then
        for root_dir in "$overrides/config/etc"/*/; do
            [[ -d "$root_dir" ]] || continue
            config_root=$(basename "$root_dir")
            # Find the owning service: the one whose service.yaml
            # declares this config-root, or whose svc-id matches.
            owner=""
            for svc_dir in "$REPO/services"/*/; do
                [[ -d "$svc_dir" ]] || continue
                svc=$(basename "$svc_dir")
                if [[ "$svc" == "$config_root" ]] && [[ ! -f "$svc_dir/service.yaml" ]]; then
                    owner=$svc
                    break
                fi
                if [[ -f "$svc_dir/service.yaml" ]] && grep -q "^config-root: ${config_root}\b" "$svc_dir/service.yaml"; then
                    owner=$svc
                    break
                fi
            done
            if [[ -z "$owner" ]]; then
                printf 'error: host config override under %s has no matching service (config-root=%s)\n' \
                    "$root_dir" "$config_root" >&2
                exit 65
            fi
            while IFS= read -r -d '' file; do
                rel=${file#"$root_dir"}
                target="$host_dir/$owner/config/$rel"
                mkdir -p "$(dirname "$target")"
                mv -- "$file" "$target"
            done < <(find "$root_dir" -type f -print0)
            find "$root_dir" -type d -empty -delete 2>/dev/null || true
        done
        find "$overrides/config" -type d -empty -delete 2>/dev/null || true
    fi

    # Remove the empty overrides/ scaffold (only if everything was migrated).
    rmdir -- "$overrides" 2>/dev/null || true
done

exit 0
