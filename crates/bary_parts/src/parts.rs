use bary_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

use crate::ComputerPrototype;
use crate::DebugPortalPrototype;
use crate::DockingPortPrototype;
use crate::ExcavatorPrototype;
use crate::InventoryPrototype;
use crate::MachinePrototype;
use crate::ThrusterPrototype;

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum PartClassification {
    Cargo,
    Machine,
    Thruster,
    Auxiliary,
    DockingPort,
    Computer,
    Structure,
    Decoration,
    Other,
}

pub type PartDatabase = BTreeMap<String, PartPrototype>;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PartPrototype {
    pub name: String,
    pub mass: Mass,
    pub dims: UVec2,
    pub layer: PartLayer,
    pub excavator_data: Option<ExcavatorPrototype>,
    pub computer_data: Option<ComputerPrototype>,
    pub inventory_data: Option<InventoryPrototype>,
    pub thruster_data: Option<ThrusterPrototype>,
    pub machine_data: Option<MachinePrototype>,
    pub docking_port_data: Option<DockingPortPrototype>,
    pub debug_portal_data: Option<DebugPortalPrototype>,
}

impl PartPrototype {
    pub fn from_file(path: &Path) -> BaryResult<Self> {
        let s = std::fs::read_to_string(path)?;
        let settings: Self = serde_yaml::from_str(&s)?;
        Ok(settings)
    }

    pub fn classification(&self) -> PartClassification {
        if self.docking_port_data.is_some() {
            PartClassification::DockingPort
        } else if self.thruster_data.is_some() {
            PartClassification::Thruster
        } else if self.machine_data.is_some() {
            PartClassification::Machine
        } else if self.inventory_data.is_some() {
            PartClassification::Cargo
        } else if self.computer_data.is_some() {
            PartClassification::Computer
        } else {
            match self.layer() {
                PartLayer::Internal => PartClassification::Auxiliary,
                PartLayer::Plumbing => PartClassification::Other,
                PartLayer::Structural => PartClassification::Structure,
                PartLayer::Exterior => PartClassification::Decoration,
            }
        }
    }
}

impl PartPrototype {
    pub fn dims(&self) -> UVec2 {
        self.dims
    }

    pub fn dims_meters(&self) -> Vec2 {
        self.dims().as_vec2() / GRID_CELLS_PER_METER
    }

    pub fn part_name(&self) -> &str {
        &self.name
    }

    pub fn part_mass(&self) -> Mass {
        self.mass
    }

    pub fn layer(&self) -> PartLayer {
        self.layer
    }
}
