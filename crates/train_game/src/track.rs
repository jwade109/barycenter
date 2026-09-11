use crate::event_bus::{EventBus, TrainEvent};
use crate::terrain::*;
use crate::{bezier::BezierCurve, node::*, world::World};
use bary_core::prelude::{Components, Ent, Isometry2d, linspace_f64, rand, wrap_0_2pi_f64};
use glam::DVec2;
use log::*;
use rend::Color;
use serde::{Deserialize, Serialize};
use splines::{Interpolation, Key, Spline};
use std::collections::{BTreeMap, BTreeSet};

fn make_length_spline(bezier: &BezierCurve) -> Spline<f64, f64> {
    let points: Vec<_> = linspace_f64(0.0, 1.0, 100)
        .into_iter()
        .map(|t| {
            let p = bezier.eval(t).translation.as_dvec2();
            (t, p)
        })
        .collect();

    let mut dist = 0.0;

    let mut keys = vec![Key::new(0.0, 0.0, Interpolation::Linear)];

    for w in points.windows(2) {
        let p1 = w[0].1;
        let p2 = w[1].1;
        let t = w[1].0;
        let d = p1.distance(p2);
        dist += d;
        let key = Key::new(dist, t, Interpolation::Linear);
        keys.push(key);
    }

    Spline::from_vec(keys)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum Terminus {
    Start,
    End,
}

impl Terminus {
    pub fn other(&self) -> Terminus {
        match self {
            Self::Start => Self::End,
            Self::End => Self::Start,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TrackLocation {
    pub track_id: Ent,
    pub pos: f64,
    pub origin: Terminus,
}

impl TrackLocation {
    pub fn new(track_id: Ent, pos: f64, origin: Terminus) -> Self {
        Self {
            track_id,
            pos,
            origin,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TrackSegment {
    pub nodes: Vec<Ent>,
    pub center: Isometry2d,
    pub bezier: BezierCurve,
    pub length: f64,
    pub s_to_t: Spline<f64, f64>,
    pub lower: DVec2,
    pub upper: DVec2,
    pub chunks: BTreeSet<ChunkIndex>,
}

impl TrackSegment {
    pub fn new(node_ids: Vec<Ent>, nodes: &Components<Node>) -> Option<Self> {
        if node_ids.len() < 2 || node_ids.len() > 4 {
            return None;
        }

        let pos: Option<Vec<DVec2>> = node_ids
            .iter()
            .map(|id| nodes.get(*id).map(|n| n.pos()))
            .collect();

        let pos = pos?;
        let bezier = BezierCurve::new(pos)?;

        let length = bezier.length();

        let s_to_t = make_length_spline(&bezier);

        let center = bezier.eval(s_to_t.sample(length / 2.0).unwrap());

        let mut lower = center.translation.as_dvec2();
        let mut upper = center.translation.as_dvec2();

        let mut chunks = BTreeSet::new();

        for t in linspace_f64(0.0, 1.0, 100) {
            let p = bezier.eval(t).translation.as_dvec2();
            lower.x = lower.x.min(p.x);
            lower.y = lower.y.min(p.y);
            upper.x = upper.x.max(p.x);
            upper.y = upper.y.max(p.y);

            let ci = get_chunk_index(p);
            chunks.insert(ci);
        }

        Some(Self {
            nodes: node_ids,
            center,
            bezier,
            length,
            s_to_t,
            lower,
            upper,
            chunks,
        })
    }

    pub fn start_node(&self) -> Ent {
        *self.nodes.first().unwrap()
    }

    pub fn end_node(&self) -> Ent {
        *self.nodes.last().unwrap()
    }

    pub fn end_nodes(&self) -> [Ent; 2] {
        [self.start_node(), self.end_node()]
    }

    pub fn s_to_t(&self, s: f64) -> f64 {
        self.s_to_t.clamped_sample(s).unwrap()
    }

    pub fn get_terminus(&self, node_id: Ent) -> Option<Terminus> {
        if node_id == self.start_node() {
            Some(Terminus::Start)
        } else if node_id == self.end_node() {
            Some(Terminus::End)
        } else {
            None
        }
    }

    pub fn get_node_at(&self, term: Terminus) -> Ent {
        match term {
            Terminus::Start => self.start_node(),
            Terminus::End => self.end_node(),
        }
    }

    pub fn eval_at(&self, term: Terminus, offset_s: f64) -> Isometry2d {
        match term {
            Terminus::Start => self.eval(offset_s),
            Terminus::End => self.eval(self.length - offset_s),
        }
    }

    // s is the path coordinate, in [0, length], not [0, 1]
    pub fn eval(&self, s: f64) -> Isometry2d {
        let t = self.s_to_t(s);
        self.bezier.eval(t)
    }

    pub fn nearest_point(&self, p: DVec2) -> (f64, DVec2) {
        let (t, pos) = self.bezier.nearest_point(p);
        let s = t * self.length;
        (s, pos)
    }
}

pub fn despawn_track(world: &mut World, track_id: Ent) -> Option<()> {
    let track = world.segments.despawn(track_id).ok()?;
    for id in track.nodes {
        if let Ok(node) = world.nodes.try_get_mut(id) {
            node.remove_track(track_id);
        }

        update_switch_node(world, id);
    }

    for chunk in track.chunks {
        chunk_deregister_track(world, chunk, track_id);
    }

    Some(())
}

pub fn spawn_new_track(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    nodes: Vec<Ent>,
) -> Option<Ent> {
    let track = TrackSegment::new(nodes.clone(), &world.nodes)?;

    let track_id = world.spawner.spawn();

    for (index, node_id) in track.nodes.iter().enumerate() {
        let node = world.nodes.try_get_mut(*node_id).ok()?;
        let role = if index == 0 {
            NodeRole::Semantic(Terminus::Start)
        } else if index + 1 == track.nodes.len() {
            NodeRole::Semantic(Terminus::End)
        } else {
            NodeRole::Geometric
        };
        node.add_track(track_id, role);
    }

    for chunk in &track.chunks {
        chunk_register_track(world, events, *chunk, track_id);
    }

    world.segments.spawn(track_id, track);

    for node_id in nodes {
        update_switch_node(world, node_id);
    }

    Some(track_id)
}

pub fn pathfind(world: &World, start: Ent, target: Ent) -> Option<Route> {
    info!("Navigating from {start} to {target}");

    let target_loc = world.nodes.get(target)?.pos();

    let mut open_set = Vec::from([(0, start)]);
    let mut closed_set = BTreeSet::new();
    let mut previous = BTreeMap::new();

    let mut success = false;

    while !open_set.is_empty() {
        let (_dist, current_node_id) = open_set.remove(0);

        if closed_set.contains(&current_node_id) {
            continue;
        }

        closed_set.insert(current_node_id);

        let node = world.nodes.get(current_node_id)?;

        let d = node.pos().distance(target_loc);

        info!("Investigating {current_node_id} (dist {d:0.1})");

        if current_node_id == target {
            info!("Got to target!");
            success = true;
            break;
        }

        for (track_id, role) in node.tracks() {
            if *role == NodeRole::Geometric {
                continue;
            }
            let track = world.segments.get(*track_id)?;
            let [a, b] = track.end_nodes();

            let next = if a == current_node_id { b } else { a };
            let nn = world.nodes.get(next)?;
            let d = (nn.pos().distance(target_loc) * 1000.0).round() as u64;

            if !previous.contains_key(&next) {
                previous.insert(next, (current_node_id, *track_id));
            }
            open_set.push((d, next));
        }

        open_set.sort();
    }

    if !success {
        return None;
    }

    let mut tracks = vec![];
    let mut cursor = target;

    while cursor != start {
        let (node_id, track_id) = *previous.get(&cursor)?;
        cursor = node_id;
        tracks.push(track_id);
    }

    tracks.reverse();

    Some(Route::new(tracks))
}

pub fn spawn_three_way_junction(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    a: Ent,
    b: Ent,
    c: Ent,
) -> Option<()> {
    let na = world.nodes.get(a)?;
    let nb = world.nodes.get(b)?;
    let nc = world.nodes.get(c)?;

    let center = (na.pos() + nb.pos() + nc.pos()) / 3.0;

    let d = spawn_new_node(world, events, center);

    spawn_new_track(world, events, vec![a, d, b]);
    spawn_new_track(world, events, vec![a, d, c]);
    spawn_new_track(world, events, vec![b, d, c]);

    Some(())
}

pub fn spawn_four_way_junction(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    a: Ent,
    b: Ent,
    c: Ent,
    d: Ent,
) -> Option<()> {
    let na = world.nodes.get(a)?;
    let nb = world.nodes.get(b)?;
    let nc = world.nodes.get(c)?;
    let nd = world.nodes.get(d)?;

    let center = (na.pos() + nb.pos() + nc.pos() + nd.pos()) / 4.0;

    let m = spawn_new_node(world, events, center);

    spawn_new_track(world, events, vec![a, m, b]);
    spawn_new_track(world, events, vec![b, m, c]);
    spawn_new_track(world, events, vec![c, m, d]);
    spawn_new_track(world, events, vec![d, m, a]);

    spawn_new_track(world, events, vec![a, m, c]);
    spawn_new_track(world, events, vec![b, m, d]);

    Some(())
}

pub fn spawn_very_large_track(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    ids: &[Ent],
) -> Option<()> {
    if ids.len() < 2 {
        return None;
    }

    let mut full_ids = vec![ids[0]];

    for pair in ids.windows(2) {
        let a = world.nodes.get(pair[0])?;
        let b = world.nodes.get(pair[1])?;
        let p = (a.pos() + b.pos()) / 2.0;
        let c = spawn_new_node(world, events, p);
        full_ids.extend([c, pair[1]]);
    }

    let n = full_ids.len();

    spawn_new_track(world, events, full_ids.get(..4)?.to_vec())?;
    spawn_new_track(world, events, full_ids.get(n - 4..)?.to_vec())?;

    for i in (3..n - 4).step_by(2) {
        spawn_new_track(world, events, full_ids.get(i..i + 3)?.to_vec())?;
    }

    Some(())
}

pub fn move_node(world: &mut World, node_id: Ent, pos: DVec2) -> Option<()> {
    let node = world.nodes.try_get_mut(node_id).ok()?;
    node.set_pos(pos);

    let node = world.nodes.get(node_id)?;

    for (track_id, _role) in node.tracks() {
        let old_track = world.segments.try_get_mut(*track_id).ok()?;
        if let Some(tr) = TrackSegment::new(old_track.nodes.clone(), &world.nodes) {
            *old_track = tr;
        } else {
            info!("Failed to update track {track_id}");
        }
    }

    Some(())
}

pub fn spawn_random_track_extension(
    world: &mut World,
    events: &mut EventBus<TrainEvent>,
    track_id: Ent,
    term: Terminus,
) -> Option<Ent> {
    let track = world.segments.get(track_id)?;

    let end_node_id = track.get_node_at(term);
    let start_pose = track.eval_at(term, 0.0);

    let start_pose = start_pose
        + match term {
            Terminus::Start => Isometry2d::new(DVec2::ZERO, bary_core::prelude::PI_64),
            _ => Isometry2d::ZERO,
        };

    let base_length = track.length;

    let length = (base_length * rand(0.95, 1.05) as f64).max(10.0);
    let cross = length * rand(-0.2, 0.2) as f64;

    let im_pos = start_pose.offset((length * 0.6, 0.0));
    let end_pose = start_pose.offset((length, cross));

    let new_a = spawn_new_node(world, events, im_pos.tr());
    let new_b = spawn_new_node(world, events, end_pose.tr());

    spawn_new_track(world, events, vec![end_node_id, new_a, new_b])
}

pub struct Route {
    segments: Vec<Ent>,
}

impl Route {
    pub fn new(segments: Vec<Ent>) -> Self {
        Self { segments }
    }

    pub fn segments(&self) -> impl Iterator<Item = &Ent> {
        self.segments.iter()
    }
}
