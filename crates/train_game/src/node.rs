use crate::{event_bus::{EventBus, TrainEvent}, terrain::*, track::*, world::World};
use bary_core::prelude::{Ent, Isometry2d, randint, wrap_0_2pi_f64};
use glam::DVec2;
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeRole {
    Semantic(Terminus),
    Geometric,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum SwitchState {
    Left,
    Middle,
    Right,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SwitchSide {
    pub tracks: Vec<(Ent, Terminus)>,
    pub active_slot: usize,
}

impl SwitchSide {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            active_slot: 0,
        }
    }

    pub fn set_active_pos(&mut self, pos: usize) -> bool {
        if self.tracks.len() > pos {
            self.active_slot = pos;
            true
        } else {
            false
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Ent, Terminus)> {
        self.tracks.clone().into_iter()
    }

    pub fn has_track(&self, track_id: Ent) -> bool {
        self.iter().any(|e| e.0 == track_id)
    }

    pub fn active(&self) -> Option<(Ent, Terminus)> {
        self.tracks.get(self.active_slot).cloned()
    }

    pub fn push(&mut self, track_id: Ent, term: Terminus) {
        self.tracks.push((track_id, term));
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    pub fn randomize(&mut self) {
        if self.tracks.is_empty() {
            self.active_slot = 0;
            return;
        }
        let n = self.tracks.len();
        let n = randint(0, n as i32);
        self.active_slot = n as usize;
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Junction {
    forward: SwitchSide,
    backward: SwitchSide,
}

impl Junction {
    pub fn new() -> Self {
        Self {
            forward: SwitchSide::new(),
            backward: SwitchSide::new(),
        }
    }

    pub fn clear(&mut self) {
        self.forward.clear();
        self.backward.clear();
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Node {
    pos: DVec2,
    angle: f64,
    tracks: BTreeMap<Ent, NodeRole>,
    junction: Junction,
}

impl Node {
    pub fn new(pos: DVec2) -> Self {
        Self {
            pos,
            angle: 0.0,
            tracks: BTreeMap::new(),
            junction: Junction::new(),
        }
    }

    pub fn pos(&self) -> DVec2 {
        self.pos
    }

    pub fn set_pos(&mut self, p: DVec2) {
        self.pos = p;
    }

    pub fn angle(&self) -> f64 {
        self.angle
    }

    pub fn set_angle(&mut self, a: f64) {
        self.angle = a;
    }

    pub fn isometry(&self) -> Isometry2d {
        Isometry2d::new(self.pos.as_vec2(), self.angle as f32)
    }

    pub fn is_semantic(&self) -> bool {
        self.tracks
            .iter()
            .any(|t| matches!(t.1, NodeRole::Semantic(_)))
    }

    pub fn linked_tracks(&self) -> impl Iterator<Item = &Ent> {
        self.tracks
            .iter()
            .filter_map(|(id, role)| matches!(role, NodeRole::Semantic(_)).then_some(id))
    }

    pub fn is_switch(&self) -> bool {
        self.linked_tracks().count() > 2
    }

    pub fn tracks(&self) -> &BTreeMap<Ent, NodeRole> {
        &self.tracks
    }

    pub fn add_track(&mut self, id: Ent, role: NodeRole) {
        self.tracks.insert(id, role);
    }

    pub fn remove_track(&mut self, id: Ent) {
        self.tracks.remove(&id);
    }

    pub fn forward(&self) -> &SwitchSide {
        &self.junction.forward
    }

    pub fn backward(&self) -> &SwitchSide {
        &self.junction.backward
    }
}

pub fn spawn_new_node(world: &mut World, events: &mut EventBus<TrainEvent>, pos: DVec2) -> Ent {
    let node = Node::new(pos);
    let id = world.spawner.spawn();
    chunk_register_node(world, events, get_chunk_index(pos), id);
    world.nodes.spawn(id, node);
    id
}

pub fn despawn_node(world: &mut World, id: Ent) {
    if let Ok(node) = world.nodes.despawn(id) {
        chunk_deregister_node(world, get_chunk_index(node.pos), id);
        for (id, _role) in node.tracks {
            _ = despawn_track(world, id);
        }
    }
}

pub fn get_next_track(world: &World, at_node: Ent, source_track: Ent) -> Option<(Ent, Terminus)> {
    let node = world.nodes.get(at_node)?;

    let select = |side: &SwitchSide| side.active();

    if node.junction.forward.has_track(source_track) {
        return select(&node.junction.backward);
    }

    if node.junction.backward.has_track(source_track) {
        return select(&node.junction.forward);
    }

    None
}

pub fn randomize_switch_state(world: &mut World, node_id: Ent) -> Option<()> {
    let node = world.nodes.try_get_mut(node_id).ok()?;

    node.junction.forward.randomize();
    node.junction.backward.randomize();

    Some(())
}

pub fn update_switch_node(world: &mut World, node_id: Ent) -> Option<()> {
    let node = world.nodes.try_get_mut(node_id).ok()?;
    if !node.is_semantic() {
        return None;
    }

    node.junction.clear();

    let mut sum_angle = 0.0;

    let mut sample_points = Vec::new();

    for track_id in node.linked_tracks() {
        let track = world.segments.get(*track_id)?;
        let term = track.get_terminus(node_id)?;
        let iso = track.eval_at(term, 0.0);
        let nearby = track.eval_at(term, 1.0).translation.as_dvec2();
        let angle = wrap_0_2pi_f64(iso.rotation as f64).to_degrees();
        let angle = angle % 180.0;

        sample_points.push((*track_id, nearby, term));

        sum_angle += angle;
    }

    sum_angle /= node.linked_tracks().count() as f64;

    node.set_angle(sum_angle.to_radians());

    let iso = node.isometry();

    for (track_id, sample, term) in sample_points {
        let local = iso.in_frame(sample);

        if local.x > 0.5 {
            node.junction.forward.push(track_id, term);
        } else if local.x < -0.5 {
            node.junction.backward.push(track_id, term);
        } else {
            info!("Can't find a direction for {track_id}");
        }
    }

    Some(())
}
