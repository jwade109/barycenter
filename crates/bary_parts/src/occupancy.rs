use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct PartOccupancy {
    pub internal: Option<Ent>,
    pub structural: Option<Ent>,
    pub external: Option<Ent>,
    pub plumbing: Option<Ent>,
}

fn clear_if_equal(val: &mut Option<Ent>, other: Ent) {
    if *val == Some(other) {
        *val = None;
    }
}

impl PartOccupancy {
    pub const EMPTY: Self = Self::new();

    pub const fn new() -> Self {
        Self {
            internal: None,
            structural: None,
            external: None,
            plumbing: None,
        }
    }

    // TODO(testing) especially with to_array
    pub fn top(&self) -> Option<Ent> {
        let arr: Vec<_> = self.to_array().iter().filter_map(|e| *e).collect();
        arr.last().copied()
    }

    pub fn has_any(&self) -> bool {
        self.internal.is_some()
            || self.structural.is_some()
            || self.external.is_some()
            || self.plumbing.is_some()
    }

    pub fn contains(&self, part_id: Ent) -> bool {
        self.to_array().contains(&Some(part_id))
    }

    pub fn to_array(&self) -> [Option<Ent>; 4] {
        [self.internal, self.structural, self.external, self.plumbing]
    }

    pub fn iter(&self) -> impl Iterator<Item = (PartLayer, Ent)> + use<'_> {
        [
            self.internal.map(|e| (PartLayer::Internal, e)),
            self.structural.map(|e| (PartLayer::Structural, e)),
            self.external.map(|e| (PartLayer::Exterior, e)),
            self.plumbing.map(|e| (PartLayer::Plumbing, e)),
        ]
        .into_iter()
        .flatten()
    }

    pub fn mark_internal(&mut self, id: impl Into<Option<Ent>>) {
        self.internal = id.into();
    }

    pub fn mark_structural(&mut self, id: impl Into<Option<Ent>>) {
        self.structural = id.into();
    }

    pub fn mark_external(&mut self, id: impl Into<Option<Ent>>) {
        self.external = id.into();
    }

    pub fn mark_plumbing(&mut self, id: impl Into<Option<Ent>>) {
        self.plumbing = id.into();
    }

    pub fn mark(&mut self, layer: PartLayer, id: impl Into<Option<Ent>>) {
        match layer {
            PartLayer::Internal => self.mark_internal(id),
            PartLayer::Structural => self.mark_structural(id),
            PartLayer::Exterior => self.mark_external(id),
            PartLayer::Plumbing => self.mark_plumbing(id),
        }
    }

    pub fn at_layer(&self, layer: PartLayer) -> Option<Ent> {
        match layer {
            PartLayer::Internal => self.internal,
            PartLayer::Plumbing => self.plumbing,
            PartLayer::Structural => self.structural,
            PartLayer::Exterior => self.external,
        }
    }

    pub fn remove(&mut self, part_id: Ent) {
        clear_if_equal(&mut self.internal, part_id);
        clear_if_equal(&mut self.structural, part_id);
        clear_if_equal(&mut self.external, part_id);
        clear_if_equal(&mut self.plumbing, part_id);
    }
}
