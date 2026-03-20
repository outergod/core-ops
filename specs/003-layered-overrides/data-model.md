# Data Model: Layered Overrides for Reusable Desired State

## Entities

### Service Catalog
- **Purpose**: Shared base definitions under `services/`.
- **Fields**:
  - `service_name` (string)
  - `artifacts` (list of Quadlet/systemd unit files)
  - `base_dropins` (list of drop-in files by artifact)

### Host Declaration
- **Purpose**: Per-host selection and identity in `hosts/<host>/host.yaml`.
- **Fields**:
  - `host` (string)
  - `services` (list of service names)

### Host Overlay Set
- **Purpose**: Host-specific drop-ins under `hosts/<host>/overrides/`.
- **Fields**:
  - `host` (string)
  - `overrides` (list of drop-in files by artifact)

### Evaluated Artifact
- **Purpose**: Concrete artifact after base + drop-in evaluation.
- **Fields**:
  - `artifact_name` (string)
  - `artifact_type` (container, volume, socket)
  - `contents` (string)
  - `source_layers` (list of files applied in order)

## Relationships

- A **Service Catalog** contains many base artifacts and base drop-ins.
- A **Host Declaration** selects services from the Service Catalog.
- A **Host Overlay Set** provides drop-ins applied after base drop-ins.
- **Evaluated Artifacts** are produced by applying base artifacts + base drop-ins + host overlays.

## Validation Rules

- Host-selected services must exist in the Service Catalog.
- Drop-ins must target an existing base artifact.
- Drop-ins are applied in lexicographic order; host overlays are applied after base drop-ins.
- Evaluation must be deterministic and side-effect free.
