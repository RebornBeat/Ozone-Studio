# ZS-COMPILE review (partial), 2026-10-06

Status: PARTIAL. Read-only, by grep and reading. Nothing built or run. The fork that produced this stopped before finishing; the orchestrator completed the blocker check below. Items marked NOT CHECKED still need a read.

## Blocker from the fork: CLEARED

`ZSEI_RELATIONS` is referenced by 11 modality files and by the validator itself. It is exported:
- `assets/pipelines/shared/semantic_relations.rs:25` `pub const ZSEI_RELATIONS: &[&str]`
- `Validated` derives `Default` at line 63 (the early-return paths rely on it).

Modality files using `ZSEI_RELATIONS.join(...)`: 3D, BCI, biology, CAD, depth, electromagnetic, geospatial, haptic, hyperspectral, IMU, kinematics.

## Covered by the fork's grep (not compiled)

- `#[path]` include plus `mod semantic_relations;` present once per file, in 15 files: electromagnetic, geospatial, haptic, hyperspectral, IMU, kinematics, 3D, BCI, depth, CAD, biology, network, proteomics, radar, sonar.
- Each has `zero_shot_relations: Vec<AcceptedRelation>` and `zero_shot_rejected: Vec<RejectedRelation>`. The `#[serde(default)]` attribute was not confirmed on each.
- Validator arity used at call sites: `validate_mapped(&proposal, &entities, &source)` in electromagnetic, geospatial, haptic, hyperspectral, IMU, kinematics; `validate_structural(&proposal, &ids)` in 3D, BCI, depth, CAD, biology, network, proteomics, radar, sonar. Matches the definitions at `semantic_relations.rs:187` and `:198`.

## NOT CHECKED

- sound, thermal, `modalities/text/main.rs`: not reached by the fork.
- Serde `#[serde(default)]` on each new field: not confirmed per file.
- Struct literals: every literal must set both new fields or use `..Default::default()`. Not checked.
- Hyperspectral final save: whether the validated relations reach the stored graph. Not checked.
- Biology renamed-function fix: not re-read.
- The five update paths that were on legacy wrappers (BCI, 3D x2, biology, CAD, depth): whether they now call the validated functions. Not checked.

## Verdict

No compile blocker found in what was checked. The file-level pass/fail cannot be given until the NOT CHECKED items are read.
