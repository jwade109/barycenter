use crate::event_bus::*;
use crate::node::*;
use crate::persistence::*;
use crate::railcar::*;
use crate::render_state::RenderState;
use crate::render_state::update_chunk_texture;
use crate::terrain::*;
use crate::track::*;
use crate::viewport::Viewport;
use bary_core::prelude::*;
use bary_input::InputState;
use bary_sim::Camera;
use log::*;
use rdev::Key;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::time::Instant;

pub const MOUSEOVER_RADIUS: f64 = 35.0;

#[derive(Debug, Clone, Copy)]
pub enum HoveredEntity {
    Track(TrackLocation),
    Node(Ent),
    Car(Ent),
}

pub struct SelectionInfo {
    pub pressed_node: Option<(Ent, Instant)>,
    pub selected_nodes: Vec<Ent>,
    pub selected_track: Option<TrackLocation>,
    pub hovered_chunk: Option<ChunkIndex>,
    pub hovered: Option<HoveredEntity>,
    pub ruler_start: Option<DVec2>,
    pub cursor_origin: Terminus,
}

impl SelectionInfo {
    pub fn new() -> Self {
        Self {
            selected_nodes: Vec::new(),
            pressed_node: None,
            selected_track: None,
            hovered_chunk: None,
            hovered: None,
            ruler_start: None,
            cursor_origin: Terminus::Start,
        }
    }
}

pub struct World {
    pub ticks: u64,
    pub current_font_id: Option<Ent>,

    pub inv_id: Ent,
    pub mush_id: Ent,

    pub time: f64,
    pub show_detail: bool,

    pub camera: Camera,
    pub target_camera: Camera,

    pub spawner: EntitySpawner,
    pub nodes: Components<Node>,
    pub segments: Components<TrackSegment>,
    pub cars: Components<RailCar>,
    pub consists: Components<RailConsist>,
    pub clouds: Vec<(DVec3, f64)>,

    pub chunks: Components<TerrainChunk>,
    pub chunk_map: BTreeMap<ChunkIndex, Ent>,

    pub calculated_route: Option<Route>,
}

impl World {
    pub fn new(font_id: Ent, inv_id: Ent, mush_id: Ent) -> Self {
        let n_clouds = 500;

        let clouds = (0..n_clouds)
            .map(|_| {
                let x = rand(-400000.0, 400000.0) as f64;
                let y = rand(-400000.0, 400000.0) as f64;
                let z = rand(0.4, 1.0) as f64;
                let r = rand(6000.0, 22000.0) as f64;
                (DVec3::new(x, y, z), r)
            })
            .collect();

        Self {
            ticks: 0,
            current_font_id: Some(font_id),
            time: 0.0,
            show_detail: false,
            camera: Camera {
                isometry: Isometry2d::ZERO,
                zoom: 0.3,
            },
            target_camera: Camera {
                isometry: Isometry2d::ZERO,
                zoom: 0.28,
            },
            spawner: EntitySpawner::default(),
            nodes: Components::default(),
            segments: Components::default(),
            cars: Components::default(),
            chunks: Components::default(),
            consists: Components::default(),
            clouds,
            chunk_map: BTreeMap::new(),
            calculated_route: None,
            inv_id,
            mush_id,
        }
    }
}

pub fn update_world(
    world: &mut World,
    sel: &mut SelectionInfo,
    dt: f64,
    mouse: DVec2,
    screen_width: DVec2,
) {
    world.time += dt;
    world.ticks += 1;

    let mut needs_reparenting = Vec::new();

    for (car_id, car) in world.cars.iter_mut() {
        car.step(dt);

        let Some(track) = world.segments.get(car.segment) else {
            continue;
        };

        if car.pos > track.length || car.pos < 0.0 {
            needs_reparenting.push(*car_id);
        }
    }

    for car_id in needs_reparenting {
        update_track_parentage(world, car_id);
    }

    world.camera.isometry.translation +=
        (world.target_camera.isometry.translation - world.camera.isometry.translation) * 0.2;
    world.camera.isometry.rotation +=
        (world.target_camera.isometry.rotation - world.camera.isometry.rotation) * 0.2;
    world.camera.zoom += (world.target_camera.zoom - world.camera.zoom) * 0.08;

    let view = Viewport::new(world.camera, screen_width);

    let mouse_world = view.screen_to_world(mouse);

    sel.hovered = None;

    {
        let mut best_dist = MOUSEOVER_RADIUS;

        for (id, car) in world.cars.iter() {
            if let Some(iso) = get_car_isometry(world, *id) {
                let d = view.meters(iso.tr().distance(mouse_world));
                if d < best_dist {
                    sel.hovered = Some(HoveredEntity::Car(*id));
                    best_dist = d;
                }
            }
        }

        if sel.pressed_node.is_none() {
            for (id, node) in world.nodes.iter() {
                let node_screen = view.world_to_screen(node.pos());
                let d = node_screen.distance(mouse);
                if d < best_dist {
                    sel.hovered = Some(HoveredEntity::Node(*id));
                    best_dist = d;
                }
            }
        }

        if sel.hovered.is_none() {
            for (id, track) in world.segments.iter() {
                let (s, nearest) = track.nearest_point(mouse_world);
                let d = view.meters(nearest.distance(mouse_world));
                if d < best_dist {
                    best_dist = d;
                    let s = match sel.cursor_origin {
                        Terminus::Start => s,
                        Terminus::End => track.length - s,
                    };
                    let loc = TrackLocation::new(*id, s, sel.cursor_origin);
                    sel.hovered = Some(HoveredEntity::Track(loc));
                }
            }
        }
    }

    sel.hovered_chunk = Some(get_chunk_index(view.screen_to_world(mouse)));
}

pub fn make_world(events: &mut EventBus, font_id: Ent, inv_id: Ent, mush_id: Ent) -> World {
    let mut world: World = World::new(font_id, inv_id, mush_id);

    if load_world(&mut world, events, "train_world").is_none() {
        error!("Failed to load world");
    }

    world
}

pub fn process_input(
    world: &mut World,
    rs: &mut RenderState,
    events: &mut EventBus,
    sel: &mut SelectionInfo,
    input: &InputState,
    dt: f64,
    mouse: DVec2,
    screen_width: DVec2,
) {
    if input.is_key_pressed(Key::Minus) {
        world.target_camera.zoom /= 1.03;
    }
    if input.is_key_pressed(Key::Equal) {
        world.target_camera.zoom *= 1.03;
    }

    if input.just_pressed_debounced(Key::ControlLeft) {
        world.show_detail ^= true;
    }

    for event in input.events() {
        if let rdev::EventType::Wheel {
            delta_x: _,
            delta_y,
        } = event.event_type
        {
            if delta_y > 0 {
                world.target_camera.zoom *= 1.3;
            } else if delta_y < 0 {
                world.target_camera.zoom /= 1.3;
            }
        }
    }

    world.target_camera.zoom = world.target_camera.zoom.clamp(0.01, 200.0);

    let n = input.is_key_pressed(Key::KeyW) && !input.is_key_pressed(Key::ControlLeft);
    let w = input.is_key_pressed(Key::KeyA) && !input.is_key_pressed(Key::ControlLeft);
    let s = input.is_key_pressed(Key::KeyS) && !input.is_key_pressed(Key::ControlLeft);
    let e = input.is_key_pressed(Key::KeyD) && !input.is_key_pressed(Key::ControlLeft);

    let x_pull = -(w as i8) + e as i8;
    let y_pull = -(s as i8) + n as i8;

    let speed = 1100.0;

    let dir = DVec2::new(x_pull as f64, y_pull as f64).normalize_or_zero();

    let vel = (dir.x * world.target_camera.isometry.local_x().as_dvec2()
        + dir.y * world.target_camera.isometry.local_y().as_dvec2())
        * speed;

    let r = input.is_key_pressed(Key::KeyE);
    let l = input.is_key_pressed(Key::KeyQ);

    let angular_vel = (l as u8 as f64 - r as u8 as f64) * 2.0;

    world.target_camera.isometry.translation += (vel * dt).as_vec2() / world.target_camera.zoom;
    world.target_camera.isometry.rotation += (angular_vel * dt) as f32;

    let view = Viewport::new(world.camera, screen_width);
    let mouse_world = view.screen_to_world(mouse);

    let shift = input.is_key_pressed(Key::ShiftLeft);

    if input.just_pressed_debounced(Key::KeyC) {
        spawn_new_node(world, events, mouse_world);
    }

    if input.just_pressed_debounced(Key::KeyV) {
        let nodes: Vec<Ent> = sel.selected_nodes.clone().into_iter().collect();
        if spawn_new_track(world, events, nodes).is_none() {
            error!("Failed to spawn new track");
        }
    }

    if input.just_pressed_debounced(rdev::Key::KeyL) {
        info!("Rerendering tile");

        for chunk in world.chunks.values() {
            if let Some(id) = chunk.gpu_data() {
                update_chunk_texture(rs, world, id, chunk.index());
            }
        }
    }

    if !input.is_key_pressed(rdev::Button::Left) {
        sel.pressed_node = None;
    }

    if input.just_pressed_debounced(rdev::Button::Left) {
        if let Some(HoveredEntity::Track(id)) = sel.hovered {
            sel.selected_track = Some(id)
        } else {
            sel.selected_track = None;
        }
    }

    if input.just_pressed_debounced(rdev::Button::Left) {
        if let Some(HoveredEntity::Node(id)) = sel.hovered {
            sel.pressed_node = Some((id, Instant::now()));

            let contains = sel.selected_nodes.contains(&id);
            match (shift, contains) {
                (true, true) => {
                    sel.selected_nodes.retain(|d| *d != id);
                }
                (true, false) => {
                    sel.selected_nodes.push(id);
                }
                (false, false) => {
                    sel.selected_nodes = vec![id];
                }
                (false, true) => {
                    sel.selected_nodes.clear();
                }
            }
        } else {
            sel.selected_nodes.clear();
        }
    }

    if input.just_pressed_debounced(rdev::Button::Right) {
        if let Some(HoveredEntity::Track(loc)) = sel.hovered {
            _ = despawn_track(world, loc.track_id);
        }
        if let Some(HoveredEntity::Node(node_id)) = sel.hovered {
            _ = despawn_node(world, node_id);
        }
    }

    if input.just_pressed_debounced(Key::KeyN) {
        if let Some(ids) = sel.selected_nodes.get(0..2) {
            world.calculated_route = pathfind(world, ids[0], ids[1]);
        } else {
            world.calculated_route = None;
        }
    }

    if let Some(ids) = sel.selected_nodes.get(0..3)
        && input.just_pressed_debounced(Key::Num3)
    {
        spawn_three_way_junction(world, events, ids[0], ids[1], ids[2]);
    }

    if let Some(ids) = sel.selected_nodes.get(0..4)
        && input.just_pressed_debounced(Key::Num4)
    {
        spawn_four_way_junction(world, events, ids[0], ids[1], ids[2], ids[3]);
    }

    if input.just_pressed_debounced(Key::KeyP) {
        events.enqueue(TrainEvent::Sound);
    }

    if input.just_pressed_debounced(Key::Num5) {
        spawn_very_large_track(world, events, &sel.selected_nodes);
    }

    if input.just_pressed_debounced(Key::KeyJ) {
        for id in sel.selected_nodes.clone() {
            update_switch_node(world, id);
        }
    }

    if input.just_pressed_debounced(Key::KeyF) {
        sel.cursor_origin = sel.cursor_origin.other();
    }

    if input.just_pressed_debounced(Key::KeyU) {
        if let Some(id) = sel.hovered_chunk {
            if let Some(id) = world.chunk_map.get(&id) {
                events.enqueue(TrainEvent::ChunkUpdate(*id));
            }
        }
    }

    if input.just_pressed_debounced(Key::KeyG) {
        if let Some(loc) = sel.selected_track {
            spawn_new_consist(world, loc, randint(7, 32) as usize);
        }
    }

    if input.just_pressed_debounced(Key::KeyM) {
        if let Some(loc) = sel.selected_track {
            let term = loc.origin.other();
            if let Some(id) = spawn_random_track_extension(world, events, loc.track_id, term) {
                let loc = TrackLocation::new(id, 0.0, Terminus::Start);
                sel.selected_track = Some(loc);
            }
        }
    }

    let mouse_world = view.screen_to_world(mouse);

    if input.is_key_pressed(Key::KeyB) {
        let index = get_chunk_index(mouse_world);
        ensure_chunk_exists(world, events, index);
    }

    if input.just_pressed_debounced(Key::KeyZ) {
        sel.ruler_start = Some(mouse_world);
    }

    if !input.is_key_pressed(Key::KeyZ) {
        sel.ruler_start = None;
    }

    let now = Instant::now();
    if let Some((node_id, time)) = sel.pressed_node {
        let delta = now - time;
        if delta.as_secs_f64() > 0.6 {
            move_node(world, node_id, mouse_world);
        }
    }

    if input.is_key_pressed(Key::ControlLeft) && input.just_pressed_debounced(Key::KeyS) {
        if save_world(world, "train_world").is_none() {
            error!("Failed to save!");
        }
    }

    if input.is_key_pressed(Key::ControlLeft) && input.just_pressed_debounced(Key::KeyL) {
        if load_world(world, events, "train_world").is_none() {
            error!("Failed to load");
        }
    }
}
