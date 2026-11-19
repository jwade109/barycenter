use bary_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::Hash;

use crate::{PartInstance, PartPrototype, PipeGeometry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Deserialize, Serialize)]
pub struct PartId(u64);

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct BlueprintId(pub String, pub u32);

impl From<&str> for BlueprintId {
    fn from(value: &str) -> Self {
        Self(value.to_string(), 0)
    }
}

impl From<String> for BlueprintId {
    fn from(value: String) -> Self {
        Self(value, 0)
    }
}

impl From<(&str, u32)> for BlueprintId {
    fn from((name, version): (&str, u32)) -> Self {
        Self(name.to_string(), version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Blueprint {
    next_part_id: PartId,
    parts: BTreeMap<PartId, PartInstance>,
    pipes: BTreeMap<PartId, PipeGeometry>,
}

impl Blueprint {
    pub fn new() -> Self {
        Self {
            next_part_id: PartId(0),
            parts: BTreeMap::new(),
            pipes: BTreeMap::new(),
        }
    }

    pub fn merge(&mut self, other: &Blueprint) {
        for (_, part) in &other.parts {
            self.add_part(part.name.clone(), part.region, part.layer());
        }
        for (_, pipe) in &other.pipes {
            self.add_pipe(*pipe);
        }
    }

    pub fn from_parts(
        prototypes: Vec<(PartCoord, Rotation, PartPrototype)>,
        pipes: Vec<PipeGeometry>,
    ) -> Self {
        let mut ret = Self::new();

        for (pos, rot, proto) in prototypes {
            let region = GridRegion::new(pos, rot, proto.dims);
            let layer = proto.layer();
            ret.add_part(proto.name, region, layer);
        }

        for pipe in pipes {
            ret.add_pipe(pipe);
        }

        ret
    }

    fn get_next_part_id(&mut self) -> PartId {
        let ret = self.next_part_id;
        self.next_part_id.0 += 1;
        ret
    }

    pub fn add_part(
        &mut self,
        name: impl Into<String>,
        region: GridRegion,
        layer: PartLayer,
    ) -> PartId {
        let id = self.get_next_part_id();
        let instance = PartInstance::new(name, layer, region);
        self.parts.insert(id, instance);
        id
    }

    pub fn get_part(&self, id: PartId) -> Option<&PartInstance> {
        self.parts.get(&id)
    }

    pub fn get_pipe(&self, id: PartId) -> Option<&PipeGeometry> {
        self.pipes.get(&id)
    }

    pub fn get_part_at_layer(&self, p: PartCoord, layer: PartLayer) -> Option<PartId> {
        // TODO make this spatial lookup faster

        if layer == PartLayer::Plumbing {
            let found = self.pipes.iter().find(|(_, pipe)| pipe.contains(p));
            return found.map(|(id, _)| *id);
        }

        let found = self.parts.iter().find(|(_, instance)| {
            if layer != instance.layer() {
                return false;
            }

            let origin = instance.origin();
            let dims = instance.dims_grid().as_ivec2();
            let p = p - origin;
            let inner = p.inner();
            inner.x >= 0 && inner.y >= 0 && inner.x < dims.x && inner.y < dims.y
        });

        found.map(|(id, _)| *id)
    }

    pub fn get_part_at(&self, p: PartCoord, layer: impl Into<Option<PartLayer>>) -> Option<PartId> {
        let layer: Option<PartLayer> = layer.into();

        for part_layer in enum_iterator::reverse_all::<PartLayer>() {
            let found = self.parts.iter().find(|(_, instance)| {
                if let Some(layer) = layer {
                    if layer != instance.layer() {
                        return false;
                    }
                }

                if instance.layer() != part_layer {
                    return false;
                }

                let origin = instance.origin();
                let dims = instance.dims_grid().as_ivec2();
                let p = p - origin;
                let inner = p.inner();
                inner.x >= 0 && inner.y >= 0 && inner.x < dims.x && inner.y < dims.y
            });

            if let Some((id, _)) = found {
                return Some(*id);
            }
        }

        None
    }

    pub fn remove_part_at(&mut self, p: PartCoord, layer: impl Into<Option<PartLayer>>) -> bool {
        let layer = layer.into();

        // try to remove a pipe first
        for id in self.get_pipes_at(p) {
            if self.remove_part(id) {
                return true;
            }
        }

        if let Some(layer) = layer {
            if let Some(id) = self.get_part_at(p, layer) {
                self.remove_part(id)
            } else {
                false
            }
        } else {
            let mut layers = PartLayer::draw_order();
            layers.reverse();
            for layer in layers {
                if self.remove_part_at(p, layer) {
                    return true;
                }
            }
            false
        }
    }

    pub fn remove_part(&mut self, id: PartId) -> bool {
        if self.parts.remove(&id).is_some() {
            return true;
        }
        if self.pipes.remove(&id).is_some() {
            return true;
        }
        false
    }

    pub fn clear(&mut self) {
        self.parts.clear();
        self.pipes.clear();
    }

    pub fn parts(&self) -> impl Iterator<Item = (&PartId, &PartInstance)> + use<'_> {
        self.parts.iter()
    }

    pub fn pipes(&self) -> impl Iterator<Item = (&PartId, &PipeGeometry)> + use<'_> {
        self.pipes.iter()
    }

    pub fn bounding_radius(&self) -> f64 {
        // BIG TODO
        50.0
    }

    pub fn part_count(&self) -> usize {
        self.parts.len()
    }

    pub fn pipe_count(&self) -> usize {
        self.pipes.len()
    }

    pub fn add_pipe(&mut self, pipe: PipeGeometry) -> PartId {
        let id = self.get_next_part_id();
        self.pipes.insert(id, pipe);
        id
    }

    pub fn get_pipes_at(&self, p: PartCoord) -> Vec<PartId> {
        self.pipes
            .iter()
            .filter_map(|(id, pipe)| pipe.contains(p).then(|| *id))
            .collect()
    }

    pub fn rotate_ccw(&mut self) {
        for (_, part) in &mut self.parts {
            part.rotate_ccw();
        }

        for (_, pipe) in &mut self.pipes {
            pipe.rotate_ccw();
        }
    }

    pub fn bounds(&self) -> (PartCoord, PartCoord) {
        if self.parts.is_empty() {
            return (IVec2::ZERO.into(), IVec2::ZERO.into());
        }

        let mut lower: Option<IVec2> = None;
        let mut upper: Option<IVec2> = None;

        for (_, part) in &self.parts {
            let l = part.region.bottom_left().inner();
            let u = l + part.dims_grid().as_ivec2();

            lower = Some(
                lower
                    .map(|k| IVec2::new(k.x.min(l.x), k.y.min(l.y)))
                    .unwrap_or(l),
            );
            upper = Some(
                upper
                    .map(|k| IVec2::new(k.x.max(u.x), k.y.max(u.y)))
                    .unwrap_or(l),
            );
        }

        (lower.unwrap().into(), upper.unwrap().into())
    }

    pub fn shift(&mut self, delta: impl Into<PartCoord>) {
        let delta = delta.into();
        self.parts.iter_mut().for_each(|(_, p)| {
            p.region += delta;
        });

        self.pipes.iter_mut().for_each(|(_, p)| {
            p.start += delta;
            p.end += delta;
        });
    }

    pub fn center(&self) -> PartCoord {
        let (lower, upper) = self.bounds();
        PartCoord::new((upper.inner() + lower.inner()) / 2)
    }

    pub fn dims(&self) -> UVec2 {
        let (lower, upper) = self.bounds();
        (upper - lower).inner().as_uvec2()
    }

    pub fn normalize_coordinates(&mut self) {
        if self.parts.is_empty() {
            return;
        }
        let (lower, _upper) = self.bounds();
        self.shift(-lower.0);
    }

    // for each pipe, returns that pipe's ID, and the ID of the part
    // each end is connected to
    pub fn pipe_connections(&self) -> Vec<(PartId, PartId, PartId)> {
        let mut ret = Vec::new();
        for (id, pipe) in &self.pipes {
            let Some(s) = self.get_part_at(pipe.start, PartLayer::Internal) else {
                continue;
            };
            let Some(e) = self.get_part_at(pipe.end, PartLayer::Internal) else {
                continue;
            };
            ret.push((*id, s, e));
        }
        ret
    }

    pub fn occupied(&self, p: impl Into<PartCoord>, layer: PartLayer) -> Option<PartId> {
        let p = p.into();
        self.parts
            .iter()
            .find_map(|(id, part)| (part.region.contains(p) && part.layer == layer).then_some(*id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_single_layer() {
        let mut bp = Blueprint::new();

        // only consider one layer for now
        let layer = PartLayer::Structural;

        //         ^
        // ........|.................
        // ........|.................
        // ........|.oooooo..........
        // ........|.oooooo..........
        // ........o------------------>
        // ..........................
        // ..........................

        let o_id = bp.add_part("o", GridRegion::new((2, 1), Rotation::East, (6, 2)), layer);

        assert_eq!(bp.occupied((1, 0), layer), None);
        assert_eq!(bp.occupied((2, 0), layer), None);
        assert_eq!(bp.occupied((3, 0), layer), None);
        assert_eq!(bp.occupied((4, 0), layer), None);
        assert_eq!(bp.occupied((5, 0), layer), None);
        assert_eq!(bp.occupied((6, 0), layer), None);
        assert_eq!(bp.occupied((7, 0), layer), None);

        assert_eq!(bp.occupied((1, 1), layer), None);
        assert_eq!(bp.occupied((2, 1), layer), Some(o_id));
        assert_eq!(bp.occupied((3, 1), layer), Some(o_id));
        assert_eq!(bp.occupied((4, 1), layer), Some(o_id));
        assert_eq!(bp.occupied((5, 1), layer), Some(o_id));
        assert_eq!(bp.occupied((6, 1), layer), Some(o_id));
        assert_eq!(bp.occupied((7, 1), layer), Some(o_id));
        assert_eq!(bp.occupied((8, 1), layer), None);

        assert_eq!(bp.occupied((1, 2), layer), None);
        assert_eq!(bp.occupied((2, 2), layer), Some(o_id));
        assert_eq!(bp.occupied((3, 2), layer), Some(o_id));
        assert_eq!(bp.occupied((4, 2), layer), Some(o_id));
        assert_eq!(bp.occupied((5, 2), layer), Some(o_id));
        assert_eq!(bp.occupied((6, 2), layer), Some(o_id));
        assert_eq!(bp.occupied((7, 2), layer), Some(o_id));
        assert_eq!(bp.occupied((8, 2), layer), None);

        assert_eq!(bp.occupied((1, 3), layer), None);
        assert_eq!(bp.occupied((2, 3), layer), None);
        assert_eq!(bp.occupied((3, 3), layer), None);
        assert_eq!(bp.occupied((4, 3), layer), None);
        assert_eq!(bp.occupied((5, 3), layer), None);
        assert_eq!(bp.occupied((6, 3), layer), None);
        assert_eq!(bp.occupied((7, 3), layer), None);

        //         ^
        // ........|........xx.......
        // ........|........xx.......
        // ........|.oooooo.xx.......
        // ........|.oooooo..........
        // ........o------------------>
        // ..........................
        // ..........................

        let x_id = bp.add_part("x", GridRegion::new((9, 2), Rotation::North, (3, 2)), layer);

        assert_eq!(bp.occupied((9, 2), layer), Some(x_id));
        assert_eq!(bp.occupied((9, 3), layer), Some(x_id));
        assert_eq!(bp.occupied((9, 4), layer), Some(x_id));
        assert_eq!(bp.occupied((10, 2), layer), Some(x_id));
        assert_eq!(bp.occupied((10, 3), layer), Some(x_id));
        assert_eq!(bp.occupied((10, 4), layer), Some(x_id));
    }

    #[test]
    fn blueprint_remove_part() {
        let mut bp = Blueprint::new();

        // only consider one layer for now
        let layer = PartLayer::Structural;

        //         ^
        // ........|.................
        // ........|.................
        // ........|.oooooo..........
        // ........|.oooooo..........
        // ........o------------------>
        // ..........................
        // ..........................

        let o_id = bp.add_part("o", GridRegion::new((2, 1), Rotation::East, (6, 2)), layer);

        assert_eq!(bp.occupied((3, 2), layer), Some(o_id));

        bp.remove_part(o_id);

        for x in -100..=100 {
            for y in -100..=100 {
                assert_eq!(bp.occupied((x, y), layer), None);
            }
        }
    }

    #[test]
    fn blueprint_shift() {
        let mut bp = Blueprint::new();

        // only consider one layer for now
        let layer = PartLayer::Structural;

        //         ^
        // ........|.................
        // ........|.................
        // ........|.oooooo..........
        // ........|.oooooo..........
        // ........o------------------>
        // ..........................
        // ..........................

        let o_id = bp.add_part("o", GridRegion::new((2, 1), Rotation::East, (6, 2)), layer);

        bp.shift((7, 5));

        let part = bp.get_part(o_id).expect("Expecting this to succeed");

        assert_eq!(part.region.bottom_left(), (9, 6).into());
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct NamedBlueprint {
    pub id: BlueprintId,
    pub blueprint: Blueprint,
}
