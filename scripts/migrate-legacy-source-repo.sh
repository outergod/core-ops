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

# Extract the `config-root` value from a service.yaml as a literal
# string (no regex interpretation). Returns empty if the file is
# missing or the key is absent. Replaces grep-based matching that
# would interpolate user data into a regex (a `.` in an identifier
# would otherwise act as a wildcard match).
service_config_root() {
    local manifest=$1
    [[ -f "$manifest" ]] || return 0
    awk '
        /^config-root:[[:space:]]*/ {
            sub(/^config-root:[[:space:]]*/, "")
            sub(/[[:space:]]+$/, "")
            print
            exit
        }
    ' "$manifest"
}

# migrate_host_dropin <dropin_dir> <host_dir>
# Moves a single host-level <unit>.<ext>.d/ directory into
# hosts/<h>/<svc>/{quadlet,systemd}/<dropin_name>/, resolving the
# owning service via unit_owner and the payload kind via the unit's
# extension. Used by both legacy host-overlay shapes
# (overrides/<unit>.<ext>.d/ and overrides/quadlet/<unit>.<ext>.d/).
migrate_host_dropin() {
    local dropin_dir=$1
    local target_host_dir=$2
    local dropin_name
    dropin_name=$(basename "$dropin_dir")
    local unit="${dropin_name%.d}"
    local unit_ext="${unit##*.}"
    local owner
    owner=$(unit_owner "$unit") || {
        local rc=$?
        if [[ $rc -eq 1 ]]; then
            printf 'error: host drop-in %s references unit %s which is not owned by any service\n' \
                "$dropin_dir" "$unit" >&2
        fi
        exit 65
    }
    local kind_dir
    if is_systemd_ext "$unit_ext"; then
        kind_dir="systemd"
    elif is_quadlet_ext "$unit_ext"; then
        kind_dir="quadlet"
    else
        printf 'error: host drop-in %s targets unit with unrecognized extension .%s\n' \
            "$dropin_dir" "$unit_ext" >&2
        exit 65
    fi
    local target_dir="$target_host_dir/$owner/$kind_dir/$dropin_name"
    mkdir -p "$target_dir"
    local conf
    for conf in "$dropin_dir"/*; do
        [[ -f "$conf" ]] || continue
        mv -- "$conf" "$target_dir/$(basename "$conf")"
    done
    rmdir -- "$dropin_dir"
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

    # 1.b quadlet-overrides/<unit>.<ext>.d/* → {quadlet,systemd}/<unit>.<ext>.d/*
    # Drop-ins must follow their base unit. Phase 1.a already moved
    # *.socket / *.mount / *.timer / *.target / *.path / *.automount
    # from quadlet/ to systemd/, so a `traefik.socket.d` drop-in
    # belongs at services/<svc>/systemd/traefik.socket.d/, not under
    # quadlet/. Routing all drop-ins to quadlet/ would produce a tree
    # the new parser rejects via cross-kind validation.
    if [[ -d "$svc_dir/quadlet-overrides" ]]; then
        for dropin_dir in "$svc_dir/quadlet-overrides"/*.d; do
            [[ -d "$dropin_dir" ]] || continue
            dropin_name=$(basename "$dropin_dir")
            unit="${dropin_name%.d}"
            unit_ext="${unit##*.}"
            if is_systemd_ext "$unit_ext"; then
                target_root="$svc_dir/systemd"
            elif is_quadlet_ext "$unit_ext"; then
                target_root="$svc_dir/quadlet"
            else
                printf 'error: drop-in %s targets unit with unrecognized extension .%s\n' \
                    "$dropin_dir" "$unit_ext" >&2
                exit 65
            fi
            mkdir -p "$target_root/$dropin_name"
            for conf in "$dropin_dir"/*; do
                [[ -f "$conf" ]] || continue
                mv -- "$conf" "$target_root/$dropin_name/$(basename "$conf")"
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
                    # Already migrated or hand-authored; sanity-check the value
                    # via fixed-string comparison (regex would mistreat dots).
                    declared=$(service_config_root "$manifest")
                    if [[ "$declared" != "$config_root" ]]; then
                        printf 'error: %s exists but declares config-root=%q (expected %q)\n' \
                            "$manifest" "$declared" "$config_root" >&2
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

    # 2.a host drop-ins. Two legacy shapes exist in the wild:
    #   (i)   hosts/<h>/overrides/<unit>.<ext>.d/         (spec-003 original)
    #   (ii)  hosts/<h>/overrides/quadlet/<unit>.<ext>.d/ (intermediate)
    # Both route via migrate_host_dropin to
    # hosts/<h>/<svc>/{quadlet,systemd}/<unit>.<ext>.d/ — drop-ins
    # follow their base unit's payload kind.

    # (i) bare overrides/<unit>.<ext>.d
    for entry in "$overrides"/*; do
        name=$(basename "$entry")
        case "$name" in
            quadlet|systemd|config) continue ;;  # handled below or by 2.b
        esac
        if [[ ! -d "$entry" ]]; then
            # A file directly under overrides/ (e.g. README.md) isn't
            # a recognized legacy artifact at this scope. Fail loudly
            # rather than leave it behind for the loader to reject.
            printf 'error: unrecognized host override entry %s (expected <unit>.<ext>.d/ directories under overrides/)\n' \
                "$entry" >&2
            exit 65
        fi
        if [[ "$name" == *.d ]]; then
            migrate_host_dropin "$entry" "$host_dir"
        else
            # A directory whose name doesn't end in `.d` is a typo
            # (`web.container.dropin/`, `web.container.d.bak/`, etc.).
            # Codex P2 on PR #28: silently skipping leaves the legacy
            # `overrides/` dir behind, the loader hard-fails on it, and
            # the operator sees "migration succeeded" plus an unrelated
            # load error. Fail loudly here.
            printf 'error: unrecognized host override directory %s (expected <unit>.<ext>.d/)\n' \
                "$entry" >&2
            exit 65
        fi
    done

    # (ii) overrides/quadlet/<unit>.<ext>.d
    if [[ -d "$overrides/quadlet" ]]; then
        for dropin_dir in "$overrides/quadlet"/*.d; do
            [[ -d "$dropin_dir" ]] || continue
            migrate_host_dropin "$dropin_dir" "$host_dir"
        done
        rmdir -- "$overrides/quadlet" 2>/dev/null || true
    fi
    # Some legacy trees split host overrides between quadlet/ and
    # systemd/ subdirs. Treat both the same way (the routing decision
    # is per-unit, not per-source-subdir).
    if [[ -d "$overrides/systemd" ]]; then
        for dropin_dir in "$overrides/systemd"/*.d; do
            [[ -d "$dropin_dir" ]] || continue
            migrate_host_dropin "$dropin_dir" "$host_dir"
        done
        rmdir -- "$overrides/systemd" 2>/dev/null || true
    fi

    # 2.b host config overrides.
    # Collect the set of services this host selects from host.yaml so
    # ambiguous config-root matches can be filtered down. Multiple
    # services CAN share a config-root (a base + a variant); the
    # winning owner for THIS host is whichever match appears in
    # host.yaml's services list. If still ambiguous after that filter,
    # fail loudly rather than picking one implicitly.
    host_yaml="$host_dir/host.yaml"
    selected_services=()
    if [[ -f "$host_yaml" ]]; then
        while IFS= read -r line; do
            selected_services+=("$line")
        done < <(awk '/^services:/{flag=1; next} /^[a-zA-Z]/{flag=0} flag && /^  - /{sub(/^  - /, ""); print}' "$host_yaml")
    fi
    if [[ -d "$overrides/config/etc" ]]; then
        for root_dir in "$overrides/config/etc"/*/; do
            [[ -d "$root_dir" ]] || continue
            config_root=$(basename "$root_dir")
            # Collect every service whose svc-id or service.yaml's
            # config-root matches.
            matches=()
            for svc_dir in "$REPO/services"/*/; do
                [[ -d "$svc_dir" ]] || continue
                svc=$(basename "$svc_dir")
                if [[ "$svc" == "$config_root" ]] && [[ ! -f "$svc_dir/service.yaml" ]]; then
                    matches+=("$svc")
                    continue
                fi
                if [[ -f "$svc_dir/service.yaml" ]]; then
                    declared=$(service_config_root "$svc_dir/service.yaml")
                    if [[ "$declared" == "$config_root" ]]; then
                        matches+=("$svc")
                    fi
                fi
            done
            if [[ ${#matches[@]} -eq 0 ]]; then
                printf 'error: host config override under %s has no matching service (config-root=%s)\n' \
                    "$root_dir" "$config_root" >&2
                exit 65
            fi
            # Narrow ambiguous matches by host.yaml selection.
            if [[ ${#matches[@]} -gt 1 ]] && [[ ${#selected_services[@]} -gt 0 ]]; then
                narrowed=()
                for candidate in "${matches[@]}"; do
                    for selected in "${selected_services[@]}"; do
                        if [[ "$candidate" == "$selected" ]]; then
                            narrowed+=("$candidate")
                            break
                        fi
                    done
                done
                if [[ ${#narrowed[@]} -gt 0 ]]; then
                    matches=("${narrowed[@]}")
                fi
            fi
            if [[ ${#matches[@]} -ne 1 ]]; then
                printf 'error: host config override under %s is ambiguous (config-root=%s matched: %s)\n' \
                    "$root_dir" "$config_root" "${matches[*]}" >&2
                printf 'rename one of the colliding services or split the override by service before re-running migration\n' >&2
                exit 65
            fi
            owner=${matches[0]}
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

    # Remove the empty overrides/ scaffold. Failing here means some
    # legacy artifact wasn't migrated and the loader will reject the
    # tree — surface that as a script failure rather than hiding it
    # behind a successful exit.
    if [[ -d "$overrides" ]]; then
        if ! rmdir -- "$overrides" 2>/dev/null; then
            printf 'error: %s is non-empty after migration; remaining content:\n' \
                "$overrides" >&2
            find "$overrides" -mindepth 1 -maxdepth 2 >&2
            exit 65
        fi
    fi
done

exit 0
