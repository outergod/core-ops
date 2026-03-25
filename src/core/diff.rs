use std::collections::BTreeSet;

use crate::core::types::{DiffItem, DiffKind, Workload};

pub fn diff_workloads(desired: &[Workload], observed: &[Workload]) -> Vec<DiffItem> {
    let desired_map = index_by_unit_name(desired);
    let observed_map = index_by_unit_name(observed);

    let mut names = BTreeSet::new();
    for name in desired_map.keys() {
        names.insert(name.clone());
    }
    for name in observed_map.keys() {
        names.insert(name.clone());
    }

    let mut diffs = Vec::new();
    for name in names {
        let desired_item = desired_map.get(&name).cloned();
        let observed_item = observed_map.get(&name).cloned();
        let kind = match (&desired_item, &observed_item) {
            (Some(desired_wl), Some(observed_wl)) => {
                if desired_wl == observed_wl {
                    continue;
                }
                DiffKind::Change
            }
            (Some(_), None) => DiffKind::Add,
            (None, Some(_)) => DiffKind::Remove,
            (None, None) => continue,
        };

        diffs.push(DiffItem {
            name,
            kind,
            desired: desired_item,
            observed: observed_item,
        });
    }

    diffs
}

pub fn diff_contains_mount_workloads(diffs: &[DiffItem]) -> bool {
    diffs.iter().any(|diff| {
        diff.desired
            .as_ref()
            .or(diff.observed.as_ref())
            .map(|workload| {
                matches!(
                    workload.quadlet_type,
                    crate::core::types::QuadletType::Mount
                        | crate::core::types::QuadletType::Automount
                )
            })
            .unwrap_or(false)
    })
}

fn index_by_unit_name(workloads: &[Workload]) -> std::collections::BTreeMap<String, Workload> {
    let mut map = std::collections::BTreeMap::new();
    for workload in workloads {
        map.insert(workload.systemd_unit_name.clone(), workload.clone());
    }
    map
}
