#![allow(unused)]

use crate::{Blueprint, PartId};
use bary_core::prelude::*;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub struct GraphNode {
    pub pos: Vec2,
    pub vel: Vec2,
    pub real_pos: PartCoord,
    pub id: PartId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeKey(pub PartId);

#[derive(Default, Debug)]
pub struct InventoryGraph {
    pub nodes: BTreeMap<NodeKey, GraphNode>,
    pub edges: HashSet<(NodeKey, NodeKey)>,
}

impl InventoryGraph {
    pub fn from_blueprint(bp: &Blueprint) -> Self {
        let mut nodes = BTreeMap::new();
        let mut edges = HashSet::new();
        for (id, part) in bp.parts() {
            let center = part.origin().inner() + part.dims_grid().as_ivec2() / 2;

            let node = GraphNode {
                pos: randvec(10.0, 100.0),
                vel: Vec2::ZERO,
                real_pos: PartCoord::new(center),
                id: *id,
            };

            let id = NodeKey(*id);

            nodes.insert(id, node);
        }

        for (_, a, b) in bp.pipe_connections() {
            let Some((na, _)) = nodes.iter().find(|(_, n)| n.id == a) else {
                continue;
            };
            let Some((nb, _)) = nodes.iter().find(|(_, n)| n.id == b) else {
                continue;
            };

            edges.insert((*na, *nb));
            edges.insert((*nb, *na));
        }

        Self { nodes, edges }
    }

    pub fn update(&mut self, mut other: InventoryGraph) {
        for (id, node) in &mut other.nodes {
            if let Some(n) = self.nodes.get(id) {
                node.pos = n.pos;
                node.vel = n.vel;
            }
        }

        *self = other;
    }

    pub fn randomize_positions(&mut self) {
        for node in self.nodes.values_mut() {
            node.pos = randvec(10.0, 200.0);
        }
    }

    pub fn step(&mut self, dt: f32) {
        let mut ids: Vec<_> = self.nodes.keys().cloned().collect();
        ids.sort();

        for i in &ids {
            let Some(node_a) = self.nodes.get(i) else {
                continue;
            };

            let pos_a = node_a.pos;

            let mut repulsion_force = Vec2::ZERO;
            let mut connected_force = Vec2::ZERO;

            const KR: f32 = 9.0;
            const KA: f32 = 1.0;
            const KC: f32 = 15.0;
            const KP: f32 = 4.0;
            const REPULSION_RADIUS: f32 = 400.0;
            const DESIRED_CONN_RADIUS: f32 = 30.0;

            for j in &ids {
                if i == j {
                    continue;
                }

                let Some(node_b) = self.nodes.get(j) else {
                    continue;
                };

                let pos_b = node_b.pos;

                let delta = pos_b - pos_a;

                if delta.length() < REPULSION_RADIUS && delta.length() > 1.0 {
                    let dir = delta.normalize_or_zero();
                    let mag = KR * REPULSION_RADIUS / delta.length();
                    repulsion_force -= dir * mag;
                }

                if self.edges.contains(&(*i, *j)) {
                    let dir = delta.normalize_or_zero();
                    let mag = KC * (delta.length() - DESIRED_CONN_RADIUS);
                    connected_force += dir * mag;
                }
            }

            let Some(node_a) = self.nodes.get_mut(i) else {
                continue;
            };

            let real_pos = node_a.real_pos.to_meters() * 25.0;

            let attraction_force = KA * -pos_a;
            let real_pos_force = KP * (real_pos - pos_a);

            node_a.vel += repulsion_force * dt;
            node_a.vel += attraction_force * dt;
            node_a.vel += connected_force * dt;
            node_a.vel += real_pos_force * dt;
            node_a.pos += node_a.vel * dt;

            node_a.vel *= 0.98;
        }
    }
}
