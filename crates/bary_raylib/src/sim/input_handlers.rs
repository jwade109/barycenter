use crate::editor_state::EditorState;
use crate::sim::*;
use crate::sounds::*;
use bary_core::prelude::*;
use bary_input::*;
use bary_sim::Camera;
use bary_sim::*;
use early_returns::*;
use log::*;

pub fn drive_ship_on_enter(client: &mut ClientSpecificInfo, world: &World) -> Option<WorldDelta> {
    let player_id = client.player_id?;
    let free = client.scene_graph.free()?;
    let grid_id = free.selected_vehicles.first().map(|s| s.grid_id)?;
    let player = world.players.try_get(player_id).ok()?;

    if player.is_driving() {
        Some(WorldDelta::PlayerPilotingExitGrid(player_id))
    } else {
        Some(WorldDelta::PlayerPilotingEnterGrid { player_id, grid_id })
    }
}

pub fn command_selected_ships_to_waypoint(
    client: &ClientSpecificInfo,
    p: DVec2,
    q: DVec2,
) -> Vec<WorldDelta> {
    let mut deltas = Vec::new();

    let Some(free) = client.scene_graph.free() else {
        return deltas;
    };

    let n_ships = free.selected_vehicles.len();

    for (i, loc) in free.selected_vehicles.iter().enumerate() {
        let s = if n_ships == 1 {
            1.0
        } else {
            i as f64 / (n_ships - 1) as f64
        };

        let waypont = p.lerp(q, s);
        let rotation = DVec2::X.angle_to(q - p);
        let waypoint = Isometry2d::new(waypont, rotation);

        let delta = WorldDelta::SetWaypoint {
            grid_id: loc.grid_id,
            waypoint,
        };

        deltas.push(delta);
    }

    deltas
}

pub fn editor_copy_on_control_c(world: &World, client: &mut ClientSpecificInfo) {
    if !client.input.is_key_pressed(Key::ControlLeft) {
        return;
    }

    let editor = some_or_return!(client.scene_graph.editor());
    let grid = some_or_return!(world.grids.get(editor.vehicle));

    let s = format!("TODO Ctrl-C behavior: {}", grid.name);

    client.chat.log(s);
}

pub fn apply_scroll_wheel_to_camera_target(delta_y: i64, target: &mut Camera) {
    let scale = 1.15;
    if delta_y > 0 {
        target.zoom *= scale;
    } else if delta_y < 0 {
        target.zoom /= scale;
    }
}

pub fn save_world_and_alert_chat(world: &World, client: &mut ClientSpecificInfo) {
    let now = chrono::offset::Local::now();

    let filename = format!("saves/world-{}/", now.format("%Y-%m-%d-%H-%M-%S"));
    match save_world(&filename, world, true) {
        Ok(_) => {
            let s = format!("Saved to {}", filename);
            client.chat.log(s);
        }
        Err(e) => {
            let s = format!("Failed to save: {:?}", e);
            client.chat.log(s);
        }
    }
}

pub fn toggle_following_on_key_f(client: &mut ClientSpecificInfo) {
    let free = some_or_return!(client.scene_graph.free());
    let grid_id = some_or_return!(free.selected_vehicles.first().map(|s| s.grid_id));

    if client.scene_graph.look_at(grid_id) {
        client.sounds.push(SoundEffect::Follow);
        debug!("Following {}", grid_id);
    }
}

pub fn reset_camera_on_ctrl_r(client: &mut ClientSpecificInfo) {
    if client.input.is_key_pressed(Key::ControlLeft) {
        debug!("Reset camera");
        client.target_camera.isometry.translation = DVec2::ZERO;
        client.target_camera.isometry.rotation = 0.0;
        client.target_camera.zoom = 8.0;
    }
}

pub fn lock_rotation_on_key_r(client: &mut ClientSpecificInfo) {
    if client.input.is_key_pressed(Key::ControlLeft) {
        return;
    }
    if let SceneGraph::Free(fly) = &mut client.scene_graph {
        debug!("Toggle lock rotation");
        fly.lock_rotation ^= true;
    }
}

pub fn enter_ship_editor(
    world: &World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
) {
    let free = some_or_return!(client.scene_graph.free());
    let grid_id = some_or_return!(free.selected_vehicles.first().map(|s| s.grid_id));
    let grid = ok_or_return!(world.grids.try_get(grid_id));

    let centroid = grid.centroid();

    client.scene_graph = SceneGraph::Editor(EditorState {
        vehicle: grid_id,
        target_coord: PartCoord::ZERO,
        actual_offset: centroid.as_vec2(),
        camera_rotation: Rotation::East,
        prototype_id: None,
        part_rotation: Rotation::East,
        layer: Some(PartLayer::Internal),
        select_start: None,
        hovered: None,
        vehicle_name_field: None,
    });

    client.target_camera.zoom = client.target_camera.zoom.max(40.0);

    client.chat.log("Switched to ship editor");
    sounds.push(SoundEffect::OpenEditor);
}
