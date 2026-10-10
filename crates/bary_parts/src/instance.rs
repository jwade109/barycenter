use crate::*;
use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PartInstance {
    pub name: String,
    pub layer: PartLayer,
    pub region: GridRegion,
}

impl PartInstance {
    pub fn new(name: impl Into<String>, layer: PartLayer, region: GridRegion) -> Self {
        Self {
            name: name.into(),
            layer,
            region,
        }
    }

    pub fn from_prototype(proto: &PartPrototype, pos: PartCoord, rot: Rotation) -> Self {
        let dims = proto.dims();

        let region = GridRegion::new(pos, rot, dims);

        Self {
            name: proto.name.to_string(),
            layer: proto.layer(),
            region,
        }
    }

    pub fn layer(&self) -> PartLayer {
        self.layer
    }

    pub fn dims_grid(&self) -> UVec2 {
        self.region.grid_aligned_dims().inner().as_uvec2()
    }

    pub fn dims_meters(&self) -> Vec2 {
        self.region.grid_aligned_dims().to_meters()
    }

    pub fn center_meters(&self) -> Vec2 {
        let iso = self.region.center_isometry();
        iso.translation.as_vec2()
    }

    pub fn origin(&self) -> PartCoord {
        self.region.bottom_left()
    }

    pub fn origin_meters(&self) -> Vec2 {
        self.region.bottom_left().to_meters()
    }

    pub fn set_origin(&mut self, p: IVec2) {
        self.region.set_bottom_left(p.into());
    }

    pub fn with_origin(&self, p: IVec2) -> Self {
        let mut ret = self.clone();
        ret.set_origin(p);
        ret
    }

    pub fn obb(&self, angle: f32, scale: f32, pos: Vec2) -> OBB {
        let dims = self.dims_meters();
        let center = rotate(self.origin().to_meters() + dims / 2.0, angle) * scale;
        OBB::new(
            AABB::from_arbitrary(scale * -dims / 2.0, scale * dims / 2.0),
            angle,
        )
        .offset(center + pos)
    }

    pub fn rotation(&self) -> Rotation {
        self.region.rot()
    }

    pub fn set_rotation(&mut self, rot: Rotation) {
        self.region.set_rot(rot);
    }

    pub fn upper_left(&self) -> PartCoord {
        let dims = self.dims_grid();
        self.origin() + PartCoord::new(IVec2::Y * dims.y as i32)
    }

    pub fn rotate_ccw(&mut self) {
        let upper_left = self.upper_left();
        let new_origin = IVec2::Y.rotate(upper_left.inner());
        self.set_rotation(enum_iterator::next_cycle(&self.rotation()));
        self.set_origin(new_origin);
    }

    pub fn machine_data<'a>(proto: &'a PartPrototype) -> Option<&'a MachinePrototype> {
        proto.machine_data.as_ref()
    }

    pub fn inventory_data<'a>(proto: &'a PartPrototype) -> Option<&'a InventoryPrototype> {
        proto.inventory_data.as_ref()
    }

    pub fn computer_data<'a>(proto: &'a PartPrototype) -> Option<&'a ComputerPrototype> {
        proto.computer_data.as_ref()
    }

    pub fn docking_port_data<'a>(proto: &'a PartPrototype) -> Option<&'a DockingPortPrototype> {
        proto.docking_port_data.as_ref()
    }

    pub fn excavator_data<'a>(proto: &'a PartPrototype) -> Option<&'a ExcavatorPrototype> {
        proto.excavator_data.as_ref()
    }

    pub fn thruster_data<'a>(proto: &'a PartPrototype) -> Option<&'a ThrusterPrototype> {
        proto.thruster_data.as_ref()
    }
}
