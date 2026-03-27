use std::collections::BTreeSet;

use crate::core::types::{
    DiffItem, DiffKind, DriftCategory, NormalizedManagedObject, NormalizedSnapshot,
    StructuredDriftRecord, Workload,
};

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

pub fn diff_normalized_snapshots(
    desired: &NormalizedSnapshot,
    last_applied: Option<&NormalizedSnapshot>,
    actual: &NormalizedSnapshot,
) -> Vec<StructuredDriftRecord> {
    let desired_map = index_by_object_id(&desired.objects);
    let actual_map = index_by_object_id(&actual.objects);
    let applied_map = last_applied
        .map(|snapshot| index_by_object_id(&snapshot.objects))
        .unwrap_or_default();

    let mut ids = BTreeSet::new();
    ids.extend(desired_map.keys().cloned());
    ids.extend(actual_map.keys().cloned());
    ids.extend(applied_map.keys().cloned());

    let mut drift = Vec::new();
    for object_id in ids {
        let desired_object = desired_map.get(&object_id);
        let actual_object = actual_map.get(&object_id);
        let applied_object = applied_map.get(&object_id);

        let category = match (desired_object, applied_object, actual_object) {
            (Some(desired_object), Some(applied_object), Some(actual_object))
                if desired_object == applied_object && desired_object != actual_object =>
            {
                Some(DriftCategory::ExternalDrift)
            }
            (Some(desired_object), Some(applied_object), Some(actual_object))
                if desired_object != applied_object && desired_object == actual_object =>
            {
                Some(DriftCategory::ExpectedChange)
            }
            (Some(desired_object), Some(applied_object), Some(actual_object))
                if desired_object != applied_object && desired_object != actual_object =>
            {
                Some(DriftCategory::ExpectedChange)
            }
            (Some(desired_object), Some(applied_object), None)
                if desired_object != applied_object =>
            {
                Some(DriftCategory::ExpectedChange)
            }
            (Some(_), None, Some(actual_object))
                if actual_object
                    .material_fields
                    .get("runtime_variance")
                    .map(|value| value == "tolerated")
                    .unwrap_or(false) =>
            {
                Some(DriftCategory::RuntimeVariance)
            }
            (Some(desired_object), None, Some(actual_object))
                if desired_object == actual_object =>
            {
                None
            }
            (Some(_), None, _) => Some(DriftCategory::ExpectedChange),
            (None, Some(_), Some(_)) | (None, None, Some(_)) => Some(DriftCategory::StaleResidue),
            _ => None,
        };

        if let Some(category) = category {
            let auto_action = !matches!(category, DriftCategory::RuntimeVariance);
            let attention_required = matches!(
                category,
                DriftCategory::ExternalDrift | DriftCategory::StaleResidue
            );
            drift.push(StructuredDriftRecord {
                object_id,
                category,
                comparison_basis: "three_way".to_string(),
                auto_action,
                attention_required,
                details: drift_details(desired_object, applied_object, actual_object),
            });
        }
    }

    drift
}

fn drift_details(
    desired: Option<&NormalizedManagedObject>,
    applied: Option<&NormalizedManagedObject>,
    actual: Option<&NormalizedManagedObject>,
) -> String {
    let desired_fields = desired.map(|object| object.material_fields.len()).unwrap_or(0);
    let applied_fields = applied.map(|object| object.material_fields.len()).unwrap_or(0);
    let actual_fields = actual.map(|object| object.material_fields.len()).unwrap_or(0);
    format!(
        "desired_fields={} applied_fields={} actual_fields={}",
        desired_fields, applied_fields, actual_fields
    )
}

fn index_by_object_id(
    objects: &[NormalizedManagedObject],
) -> std::collections::BTreeMap<String, NormalizedManagedObject> {
    let mut map = std::collections::BTreeMap::new();
    for object in objects {
        map.insert(object.object_id.clone(), object.clone());
    }
    map
}
