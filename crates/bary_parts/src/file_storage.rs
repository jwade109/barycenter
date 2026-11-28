use bary_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::{Blueprint, BlueprintId, PartDatabase, PartPrototype, PipeGeometry};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VehicleFileStorage {
    parts: Vec<VehiclePartFileStorage>,
    pipes: Option<Vec<PipeFileStorage>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VehiclePartFileStorage {
    partname: String,
    pos: PartCoord,
    rot: Rotation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PipeFileStorage {
    geometry: PipeGeometry,
}

pub fn load_blueprint(
    id: &BlueprintId,
    assets_dir: impl AsRef<Path>,
    parts: &PartDatabase,
) -> BaryResult<Blueprint> {
    let old_path = assets_dir
        .as_ref()
        .join("vehicles")
        .join(format!("{}.vehicle", id.0));

    let new_path = assets_dir
        .as_ref()
        .join("vehicles")
        .join(format!("{}/v{}.bp", id.0, id.1));

    if old_path.exists() {
        load_blueprint_file(old_path, parts)
    } else {
        load_blueprint_file(new_path, parts)
    }
}

pub fn load_blueprint_file(
    path: impl AsRef<Path>,
    parts: &PartDatabase,
) -> BaryResult<Blueprint> {
    let s = std::fs::read_to_string(path)?;
    let storage: VehicleFileStorage = serde_yaml::from_str(&s)?;
    let mut prototypes = Vec::new();
    for part in &storage.parts {
        let proto = parts
            .get(&part.partname)
            .ok_or(BaryError::NoPartWithName(part.partname.clone()))?;
        prototypes.push((part.pos, part.rot, proto.clone()));
    }
    let pipes = storage
        .pipes
        .unwrap_or(vec![])
        .iter()
        .map(|p| p.geometry)
        .collect();
    Ok(Blueprint::from_parts(prototypes, pipes))
}

fn part_from_path(path: &Path) -> BaryResult<PartPrototype> {
    let data_path = path.join("metadata.yaml");
    let s = std::fs::read_to_string(&data_path)?;
    Ok(serde_yaml::from_str(&s)?)
}

pub fn load_parts_from_dir(path: impl AsRef<Path>) -> BaryResult<PartDatabase> {
    let mut ret = BTreeMap::new();
    if let Ok(paths) = std::fs::read_dir(path) {
        for path in paths.flatten() {
            let path = path.path();
            let part = part_from_path(&path)?;
            ret.insert(part.part_name().to_string(), part);
        }
    }
    Ok(ret)
}

pub fn save_blueprint(path: impl AsRef<Path>, blueprint: &Blueprint) -> BaryResult<()> {
    let parts = blueprint
        .parts()
        .map(|(_, instance)| VehiclePartFileStorage {
            partname: instance.name.clone(),
            pos: instance.origin(),
            rot: instance.rotation(),
        })
        .collect();

    let pipes = blueprint
        .pipes()
        .map(|(_, pipe)| PipeFileStorage { geometry: *pipe })
        .collect();

    let storage = VehicleFileStorage {
        parts,
        pipes: Some(pipes),
    };

    let s = serde_yaml::to_string(&storage)?;
    std::fs::write(path.as_ref(), s)?;
    Ok(())
}
