use crate::constants::*;
use crate::sim::*;
use crate::sounds::*;
use crate::utils::*;
use bary_core::prelude::*;
use bary_input::*;
use bary_sim::Camera;
use bary_sim::*;
use early_returns::*;
use rdev::Button;

fn camera_zooms_with_plus_minus(input: &InputState, target: &mut Camera) {
    let zoom_scale = 1.07;

    if input.is_key_pressed(Key::Minus) {
        target.zoom /= zoom_scale;
    }
    if input.is_key_pressed(Key::Equal) {
        target.zoom *= zoom_scale;
    }
}

fn camera_moves_with_wasd(
    input: &InputState,
    target: &mut Camera,
    follow: &mut Option<Ent>,
    lock_rotation: &mut bool,
    sounds: &mut SoundEffects,
) {
    let angular_speed = 2.5f64.to_radians();
    let speed = 40.0 / target.zoom as f64;

    let old_follow = *follow;

    let right = rotate_f64(DVec2::X, target.isometry.rotation);
    let up = rotate_f64(right, PI_64 / 2.0);

    if input.is_key_pressed(Key::ControlLeft) {
        return;
    }

    if input.is_key_pressed(Key::KeyQ) {
        target.isometry.rotation += angular_speed;
        *lock_rotation = false;
    }
    if input.is_key_pressed(Key::KeyE) {
        target.isometry.rotation -= angular_speed;
        *lock_rotation = false;
    }
    if input.is_key_pressed(Key::KeyS) {
        target.isometry.translation -= up * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyW) {
        target.isometry.translation += up * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyD) {
        target.isometry.translation += right * speed;
        *follow = None;
    }
    if input.is_key_pressed(Key::KeyA) {
        target.isometry.translation -= right * speed;
        *follow = None;
    }

    if old_follow.is_some() && follow.is_none() {
        sounds.push(SoundEffect::LeaveFollow);
    }
}

fn editor_actual_offset_smooth_animation(target: Vec2, actual: &mut Vec2) {
    let rate_translation = 0.2;
    actual.x = low_pass_f32(actual.x, target.x, rate_translation);
    actual.y = low_pass_f32(actual.y, target.y, rate_translation);
}

pub fn animate_camera_towards_target(target: &Camera, actual: &mut Camera) {
    let rate_translation = 0.2;
    let rate_rotation = 0.2;
    actual.isometry.translation.x = low_pass_f64(
        actual.isometry.translation.x,
        target.isometry.translation.x,
        rate_translation,
    );
    actual.isometry.translation.y = low_pass_f64(
        actual.isometry.translation.y,
        target.isometry.translation.y,
        rate_translation,
    );
    actual.isometry.rotation = low_pass_f64(
        actual.isometry.rotation,
        target.isometry.rotation,
        rate_rotation,
    );
    actual.zoom = low_pass_f32(actual.zoom, target.zoom, rate_translation as f32);
}

fn update_actual_hover_part_info(client: &mut ClientSpecificInfo, grids: &Components<VehicleGrid>) {
    let mouse_screen_position = client.mouse_screen_position;
    let screen_dims = client.screen_dims;

    if let Some(free) = client.viewport.free_mut() {
        free.selection_info.hovered = None;
        let screen_pos = some_or_return!(mouse_screen_position);
        let world_pos = screen_to_world(&client.camera, screen_pos, screen_dims);
        let (grid_id, offset) = some_or_return!(get_closest_grid(grids, world_pos, None));
        let dist = offset.length();
        let grid = ok_or_return!(grids.try_get(grid_id));

        let grid_pos = grid.centroid_isometry().tr();
        let grid_screen = get_world_to_screen(&client.camera, grid_pos, screen_dims);
        let d_screen = grid_screen.distance(screen_pos);

        if 20.0 * (grid.bounding_radius() as f64) < dist && d_screen > 80.0 {
            return;
        }
        let origin = grid.origin();
        let coord = PartCoord::from_meters_floored(in_frame(origin, world_pos));
        free.selection_info.hovered = Some(GridLocation::new(grid_id, coord));
    } else if let Some(editor) = client.viewport.editor_mut() {
        editor.hovered = None;
        let grid = ok_or_return!(grids.try_get(editor.vehicle));
        let screen_pos = some_or_return!(mouse_screen_position);
        let world_pos = screen_to_world(&client.camera, screen_pos, screen_dims);
        // TODO(cleanup) completely unnecessary. shouldn't need to get the world coordinates
        // or the grid's coordinates to get this vector. just ask how far the camera is from
        // the grid in question!
        let local_pos = in_frame(grid.origin(), world_pos);
        let coord = PartCoord::from_meters_floored(local_pos);
        editor.hovered = Some(coord);
    }
}

fn set_target_camera_if_following(
    follow: Option<Ent>,
    lock_rotation: bool,
    grids: &Components<VehicleGrid>,
    target: &mut Camera,
    actual: &mut Camera,
) {
    let Some(follow) = follow else {
        return;
    };

    let Some(grid) = grids.get(follow) else {
        return;
    };

    let iso = grid.centroid_isometry();

    target.isometry.translation = iso.translation;
    if lock_rotation {
        target.isometry.rotation = iso.rotation;
    }

    actual.isometry.translation = target.isometry.translation;
}

fn select_hovered_grid_loc_on_click(client: &mut ClientSpecificInfo, sounds: &mut SoundEffects) {
    let free = some_or_return!(client.viewport.free_mut());
    let old_grid = free.selection_info.first_selected_grid();

    let Some(hovered) = free.selection_info.hovered else {
        free.selection_info.selected.clear();
        return;
    };

    if client.input.is_key_pressed(Key::ShiftLeft) {
        free.selection_info.selected.push(hovered);
    } else {
        free.selection_info.selected = vec![hovered];
    }

    if free.selection_info.first_selected_grid().is_some() {
        sounds.push(SoundEffect::Open);
    } else if old_grid.is_some() {
        sounds.push(SoundEffect::Close);
    }
}

fn editor_on_release_left_click(client: &mut ClientSpecificInfo) -> Option<WorldDelta> {
    let e = client.viewport.editor_mut()?;

    let src = e.select_start?;
    let dst = e.hovered?;

    e.select_start = None;

    if e.layer != Some(PartLayer::Plumbing) {
        return None;
    }

    let delta = WorldDelta::InsertPipe {
        grid_id: e.vehicle,
        src,
        dst,
    };

    Some(delta)
}

fn lock_ship(world: &World, client: &ClientSpecificInfo) -> Option<WorldDelta> {
    let primary_grid = client
        .viewport
        .free()?
        .selection_info
        .first_selected_grid()?;
    let grid = world.grids.try_get(primary_grid).ok()?;
    Some(WorldDelta::SetAnchored(primary_grid, !grid.is_anchored))
}

fn editor_on_left_click(world: &World, client: &mut ClientSpecificInfo) -> Option<WorldDelta> {
    let e = client.viewport.editor_mut()?;
    let coord = e.hovered?;

    e.select_start = None;

    if let Some(proto_id) = e.prototype_id {
        let proto = world.prototypes.try_get(proto_id).ok()?;

        Some(WorldDelta::InsertPart {
            grid_id: e.vehicle,
            name: proto.name.clone(),
            coord,
            rotation: e.part_rotation,
            layer: proto.layer,
        })
    } else {
        e.select_start = Some(coord);
        None
    }
}

pub fn process_event(
    world: &World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
    on_gui: bool,
) {
    let events: Vec<_> = client.input.events().cloned().collect();

    for event in events {
        match event.event_type {
            rdev::EventType::KeyPress(key) => match key {
                Key::KeyS => {
                    if client.input.is_key_pressed(Key::ControlLeft) {
                        input_handlers::save_world_and_alert_chat(world, client);
                    }
                }
                Key::KeyF => input_handlers::toggle_following_on_key_f(client, sounds),
                Key::KeyR => {
                    input_handlers::reset_camera_on_ctrl_r(client);
                    input_handlers::lock_rotation_on_key_r(client);
                }
                Key::KeyC => {
                    input_handlers::editor_copy_on_control_c(world, client);
                }
                _ => (),
            },
            rdev::EventType::KeyRelease(_key) => (),
            rdev::EventType::ButtonPress(button) => {
                if !on_gui {
                    match button {
                        Button::Left => {
                            select_hovered_grid_loc_on_click(client, sounds);
                        }
                        Button::Right => (),
                        Button::Middle => (),
                        Button::Unknown(_) => (),
                    }
                }
            }
            rdev::EventType::MouseMove { x: _, y: _ } => (),
            rdev::EventType::Wheel {
                delta_x: _,
                delta_y,
            } => {
                input_handlers::apply_scroll_wheel_to_camera_target(
                    delta_y,
                    &mut client.target_camera,
                );
            }
            _ => (),
        }
    }
}

fn update_terrain_selection_info(client: &mut ClientSpecificInfo, asteroids: &Components<BigRock>) {
    let free = some_or_return!(client.viewport.free_mut());
    free.hovered_chunk = None;

    let screen_pos = some_or_return!(client.mouse_screen_position);
    let world_pos = screen_to_world(&client.camera, screen_pos, client.screen_dims);

    for (rock_id, rock) in asteroids.iter() {
        let d = world_pos.distance(rock.iso.translation);
        if d > rock.ast.max_radius() as f64 {
            continue;
        }

        let rock_local = in_frame(rock.iso, world_pos);

        let tile = GlobalTileIndex(vfloor_f64(rock_local / TERRAIN_TILE_WIDTH_METERS));

        let info = TerrainSelectionInfo {
            asteroid: *rock_id,
            tile,
        };

        free.hovered_chunk = Some(info);
    }
}

fn toggle_tracking_for_selected_grid(client: &ClientSpecificInfo) -> Option<WorldDelta> {
    let free = client.viewport.free()?;
    let grid_id = free.selection_info.first_selected_grid()?;
    Some(WorldDelta::ToggleTracking(grid_id))
}

fn explode_at_mouseover(client: &ClientSpecificInfo) -> Option<WorldDelta> {
    let free = client.viewport.free()?;
    let loc = free.selection_info.hovered?;
    Some(WorldDelta::Explode(loc))
}

fn ping_on_alt_left_click(client: &ClientSpecificInfo) -> Option<WorldDelta> {
    let screen_pos = client.mouse_screen_position?;
    let pos = screen_to_world(&client.camera, screen_pos, client.screen_dims);
    Some(WorldDelta::Ping(pos))
}

fn destroy_top_layer_part_at_mouseover(client: &ClientSpecificInfo) -> Option<WorldDelta> {
    let editor = client.viewport.editor()?;
    let loc = client.hovered_grid_loc()?;
    Some(WorldDelta::DestroyPartAt {
        loc,
        layer: editor.layer,
    })
}

#[must_use]
pub fn pre_simulation_update(world: &World, client: &mut ClientSpecificInfo) -> Vec<WorldDelta> {
    client.ticks += 1;

    update_actual_hover_part_info(client, &world.grids);

    update_terrain_selection_info(client, &world.asteroids);

    if client.input.just_pressed_debounced(Key::Alt) {
        client.alt_mode ^= true;
    }

    if client.input.just_pressed_debounced(Button::Right) {
        if let Some(mouse_pos) = client.mouse_screen_position {
            if let Some(free) = client.viewport.free_mut() {
                let world_pos = screen_to_world(&client.camera, mouse_pos, client.screen_dims);
                free.waypoint_widget = Some(world_pos);
            }
        }
    }

    let mut deltas = Vec::new();

    if client.input.just_pressed_debounced(Key::KeyT) {
        if let Some(d) = toggle_tracking_for_selected_grid(client) {
            deltas.push(d);
        }
    }

    if client.input.just_pressed_debounced(Key::KeyC) {
        if let Some(d) = explode_at_mouseover(client) {
            deltas.push(d);
        }
    }

    if client.input.just_pressed_debounced(Button::Left)
        && client.input.is_key_pressed(Key::ControlLeft)
    {
        if let Some(d) = ping_on_alt_left_click(client) {
            deltas.push(d);
        }
    }

    if client.input.just_pressed(Button::Right) {
        if let Some(d) = destroy_top_layer_part_at_mouseover(client) {
            deltas.push(d);
        }
    }

    if client.input.just_pressed_debounced(Key::Return) {
        if let Some(d) = drive_ship_on_enter(client, world) {
            deltas.push(d);
        }
    }

    if client.input.just_released(Button::Right) {
        if let Some(free) = client.viewport.free() {
            if let Some(p) = free.waypoint_widget {
                if let Some(mouse_pos) = client.mouse_screen_position {
                    let q = screen_to_world(&client.camera, mouse_pos, client.screen_dims);
                    deltas.extend(input_handlers::command_selected_ships_to_waypoint(
                        client, p, q,
                    ));
                }
            }
        }

        if let Some(free) = client.viewport.free_mut() {
            free.waypoint_widget = None;
        }
    }

    deltas
}

fn zoom_in_on_key_v(client: &mut ClientSpecificInfo) {
    if !client.input.just_pressed_debounced(Key::KeyV) {
        return;
    }

    let grid_id = some_or_return!(client.focused_grid_id());
    let free = some_or_return!(client.viewport.free_mut());
    if client.target_camera.zoom < ZOOM_NEAR_VEHICLE {
        client.target_camera.zoom = ZOOM_NEAR_VEHICLE;
    } else {
        client.target_camera.zoom = ZOOM_FAR_AWAY;
    }
    free.follow_vehicle = Some(grid_id);
}

#[allow(unused)]
fn do_terrain_tile_under_mouse(
    world: &World,
    client: &mut ClientSpecificInfo,
) -> Option<WorldDelta> {
    let free = client.viewport.free()?;
    let tile_info = free.hovered_chunk?;

    let asteroid = tile_info.asteroid;
    let tile = tile_info.tile;

    let (chunk_idx, tile_idx) = tile.to_cl();

    let ast_id = asteroid;
    let ast = world.asteroids.get(ast_id)?;
    let chunk_id = ast.chunks.get(&chunk_idx)?;
    let chunk = world.terrain_chunks.get(*chunk_id)?;
    let tile_id = chunk.tiles.get(&tile_idx)?;

    // just confirm that the tile exists
    _ = world.terrain_tiles.get(*tile_id)?;

    Some(WorldDelta::FullyRevealTerrainTile { asteroid, tile })
}

#[must_use]
pub fn post_simulation_update(
    world: &World,
    client: &mut ClientSpecificInfo,
    sounds: &mut SoundEffects,
    is_terminal_focused: bool,
) -> Vec<WorldDelta> {
    client.chat.drop_old_messages();

    zoom_in_on_key_v(client);

    let mut deltas = Vec::new();

    if client.input.just_pressed(Button::Left) {
        if let Some(d) = editor_on_left_click(world, client) {
            deltas.push(d);
        }
    }

    if client.input.just_pressed(Key::KeyX) {
        if let Some(d) = lock_ship(world, client) {
            deltas.push(d);
        }
    }

    if client.input.just_released(Button::Left) {
        if let Some(d) = editor_on_release_left_click(client) {
            deltas.push(d);
        }
    }

    match &mut client.viewport {
        Viewport::Free(fly) => {
            set_target_camera_if_following(
                fly.follow_vehicle,
                fly.lock_rotation,
                &world.grids,
                &mut client.target_camera,
                &mut client.camera,
            );

            if !is_terminal_focused {
                camera_moves_with_wasd(
                    &client.input,
                    &mut client.target_camera,
                    &mut fly.follow_vehicle,
                    &mut fly.lock_rotation,
                    sounds,
                );

                camera_zooms_with_plus_minus(&client.input, &mut client.target_camera);
            }
        }
        Viewport::Editor(editor) => {
            if !is_terminal_focused {
                camera_zooms_with_plus_minus(&client.input, &mut client.target_camera);
            }

            editor_actual_offset_smooth_animation(editor.target_offset, &mut editor.actual_offset);

            set_cams_to_grid_pose(
                editor.vehicle,
                &world.grids,
                editor.actual_offset,
                &mut client.target_camera,
                &mut client.camera,
            );
        }
    }

    if client.target_camera.zoom > VIEWPORT_MAX_ZOOM {
        client.target_camera.zoom = VIEWPORT_MAX_ZOOM;
    }

    animate_camera_towards_target(&client.target_camera, &mut client.camera);

    deltas
}

fn set_cams_to_grid_pose(
    grid_id: Ent,
    grids: &Components<VehicleGrid>,
    offset: Vec2,
    target: &mut Camera,
    actual: &mut Camera,
) {
    if let Ok(grid) = grids.try_get(grid_id) {
        target.isometry = grid.origin().offset(offset);
        target.zoom = target.zoom.clamp(EDITOR_MINIMUM_ZOOM, EDITOR_MAXIMUM_ZOOM);
        actual.isometry = target.isometry;
    }
}
