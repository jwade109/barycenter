use std::collections::BTreeMap;
use std::time::Duration;

use crate::bezier::{BezierCurve, nearest_point_segment};
use crate::event_bus::{EventBus, FontSelection, TrainEvent};
use crate::railcar::RailCar;
use crate::terrain::{TERRAIN_CHUNK_WIDTH_METERS, TerrainChunk};
use crate::track::{Terminus, TrackSegment};
use crate::tweens::{AnimationStates, Tween};
use crate::viewport::Viewport;
use crate::world::*;
use bary_core::prelude::*;
use bary_input::InputState;
use rend::*;

fn draw_button(
    cmd: &mut RenderCommands,
    anim: &AnimationStates,
    text: &str,
    p: DVec2,
    mouse: DVec2,
    input: &InputState,
    id: usize,
    color: Color,
) -> (DVec2, bool) {
    let padding = DVec2::splat(15.0);
    let extent = cmd
        .text(p + padding, text)
        .size(22.0)
        .color(Color::WHITE)
        .extent();
    let extent = extent.max(DVec2::new(160.0, extent.y));
    let full_extent = extent + padding * 2.0;
    let rect_origin = p - extent.y * DVec2::Y;
    let aabb = AABB::from_arbitrary(rect_origin.as_vec2(), (rect_origin + full_extent).as_vec2());
    let contains = aabb.contains(mouse.as_vec2());

    let t = anim.anim(("button", id), Tween::Exponential, 0.1, contains);
    let alpha = lerp(0.7, 1.0, t as f32) as f64;

    let extra_extent = DVec2::new(200.0 * t, 0.0);

    let expanded_extent = full_extent + extra_extent;
    let expanded_origin = rect_origin.with_y(rect_origin.y - extra_extent.y);

    let alpha = contains as u8 as f64 * 0.2 + 0.9;

    cmd.rect(rect_origin)
        .dims(expanded_extent)
        .color(color.alpha(alpha))
        .z(0.52);
    cmd.text_with_shadow(
        p + padding,
        (-2.0, -2.0),
        text,
        22.0,
        Color::WHITE,
        Color::BLACK.alpha(0.7),
    );

    (
        full_extent,
        input.just_pressed(rdev::Button::Left) && contains,
    )
}

fn draw_terrain(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for chunk in world.chunks.values() {
        if chunk.index().as_ivec2().as_dvec2().length() > 5.0 {
            continue;
        }

        let iso = view.w2s_iso(chunk.isometry());
        let dims = DVec2::splat(view.meters(TERRAIN_CHUNK_WIDTH_METERS));
        // cmd.rect(iso).dims(dims);
        // if let Some(id) = chunk.gpu_data() {
        let p = chunk.index().as_ivec2();
        let id = if (p.x + p.y) % 2 == 0 {
            world.mush_id
        } else {
            world.inv_id
        };
        // cmd.chunk(chunk.index().as_ivec2(), iso, dims, chunk.height(), id);
        if let Some(id) = chunk.gpu_data() {
            cmd.sprite(id, iso, dims);
        }
        // }
    }
}

fn draw_grid_lines(cmd: &mut RenderCommands, view: &Viewport) {
    let n_lines = 500;

    let spacing = if view.zoom() > 20.0 {
        5
    } else if view.zoom() > 2.0 {
        25
    } else if view.zoom() > 0.2 {
        250
    } else if view.zoom() > 0.02 {
        1000
    } else {
        10000
    };

    for x in -n_lines..=n_lines {
        let x = x * spacing;
        let s = DVec2::new(x as f64, -(spacing * n_lines) as f64);
        let e = DVec2::new(x as f64, (spacing * n_lines) as f64);
        cmd.line(view.world_to_screen(s), view.world_to_screen(e))
            .color(Color::GRAY.alpha(0.5))
            .thickness(3.0);
        let s = DVec2::new(-(spacing * n_lines) as f64, x as f64);
        let e = DVec2::new((spacing * n_lines) as f64, x as f64);
        cmd.line(view.world_to_screen(s), view.world_to_screen(e))
            .color(Color::GRAY.alpha(0.5))
            .thickness(3.0);
    }
}

fn draw_font_ui(
    cmd: &mut RenderCommands,
    anim: &AnimationStates,
    events: &mut EventBus,
    font_selection: &mut FontSelection,
    mouse: DVec2,
    input: &InputState,
) {
    let fonts = cmd.fonts.clone();

    let mut p = DVec2::new(30.0, 300.0);
    for (i, (font_id, font)) in fonts.iter().enumerate() {
        let color = Color::hsl(i as f64 / 10.0, 0.3, 0.45, 0.95);

        let text = format!("{} {}", font_id, font.name);
        let (e, clicked) = draw_button(cmd, anim, &text, p, mouse, input, i, color);
        if clicked {
            font_selection.clicked(*font_id);
            events.enqueue(TrainEvent::Sound);
        }
        p.y += e.y + 15.0;
    }
}

fn draw_bezier(
    cmd: &mut RenderCommands,
    bezier: &BezierCurve,
    view: &Viewport,
    handles: bool,
    color: Color,
    thickness: f64,
) {
    if handles && !bezier.is_linear() {
        let handles = bezier.points().map(|p| view.world_to_screen(*p)).collect();
        cmd.linestring(handles).thickness(3.0).color(Color::PURPLE);
    }

    let points = bezier
        .linestring(0.0, 1.0, 30)
        .into_iter()
        .map(|p| view.world_to_screen(p.translation.as_dvec2()))
        .collect();

    cmd.linestring(points).thickness(thickness).color(color);
}

fn draw_track(
    cmd: &mut RenderCommands,
    track: &TrackSegment,
    world: &World,
    view: &Viewport,
    handles: bool,
    color: Color,
    thickness: f64,
) -> Option<()> {
    let points: Option<Vec<_>> = track
        .nodes
        .iter()
        .map(|id| world.nodes.get(*id).map(|n| n.pos()))
        .collect();
    let points = points?;
    let bezier = BezierCurve::new(points)?;
    draw_bezier(cmd, &bezier, view, handles, color, thickness);

    Some(())
}

fn draw_tracks(cmd: &mut RenderCommands, world: &World, view: &Viewport, handles: bool) {
    for segment in world.segments.values() {
        _ = draw_track(cmd, segment, world, view, handles, Color::BLACK, 6.0);
    }
}

fn draw_track_nodes(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for node in world.nodes.values() {
        let p = view.world_to_screen(node.pos());
        if node.tracks().is_empty() {
            cmd.circle(p).radii(8.0, 12.0).color(Color::GRAY);
        } else if node.is_semantic() {
            cmd.circle(p).radius(9.0).color(Color::BLUE);
        } else {
            cmd.circle(p).radius(6.0).color(Color::RED);
        }
    }
}

fn draw_track_junctions(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for node in world.nodes.values() {
        if node.is_switch() {
            let iso = node.isometry();

            let p = iso.offset(Vec2::X * 2.0).translation.as_dvec2();
            let q = iso.offset(-Vec2::X * 2.0).translation.as_dvec2();

            let p = view.world_to_screen(p);
            let q = view.world_to_screen(q);

            cmd.line(p, q)
                .thickness(view.meters(2.0))
                .color(Color::GRAY);

            let p = iso.offset(Vec2::Y * 3.0).translation.as_dvec2();
            let q = iso.offset(-Vec2::Y * 3.0).translation.as_dvec2();

            let p = view.world_to_screen(p);
            let q = view.world_to_screen(q);

            cmd.line(p, q).thickness(12.0).color(Color::RED);

            cmd.isometry(view.w2s_iso(iso), view.meters(5.0));
        }
    }
}

pub fn draw_railcar<'a>(
    cmd: &'a mut RenderCommands,
    iso: impl Into<Isometry2d>,
    view: &Viewport,
) -> RectBuilder<'a> {
    let dims = DVec2::new(
        view.meters(RailCar::LENGTH_METERS).max(11.0),
        view.meters(RailCar::WIDTH_METERS).max(11.0),
    );

    let iso = iso.into();
    let iso = view.w2s_iso(iso);
    cmd.rect(iso).dims(dims).centered()
}

fn draw_debug_info(
    cmd: &mut RenderCommands,
    world: &World,
    view: &Viewport,
    sel: &SelectionInfo,
    anim: &AnimationStates,
    draw_calls: usize,
    timers: &BTreeMap<&'static str, Duration>,
    frame_timer: &WallTimer,
) {
    let p = DVec2::new(20.0, view.dims().y - 30.0);

    let mut lines = vec![
        format!("{} ticks", world.ticks),
        format!("framerate      {:0.1} Hz", frame_timer.actual_rate()),
        format!("zoom           {:0.3}", world.camera.zoom),
        format!("hovered        {:?}", sel.hovered),
        format!("selected_nodes {:?}", sel.selected_nodes),
        format!("pressed_node   {:?}", sel.pressed_node),
        format!("selected_track {:?}", sel.selected_track),
        format!("draw_calls     {:?}", draw_calls),
    ];

    for (id, num, tween, state) in anim.animations() {
        lines.push(format!("{} {} {:?} {:0.2}", id, num, tween, state));
    }

    for (name, dur) in timers {
        lines.push(format!("{} {:?}", name, dur.as_millis()));
    }

    let text = lines.join("\n");

    for (off, color) in [(0.0, Color::WHITE)] {
        let p = p - DVec2::splat(off);
        let extent = cmd.text(p, &text).size(28.0).color(color).extent();

        cmd.rect(p - extent.y * DVec2::Y)
            .dims(extent)
            .color(Color::BLACK.alpha(0.4));
    }
}

pub fn draw_world(
    cmd: &mut RenderCommands,
    sel: &SelectionInfo,
    events: &mut EventBus,
    fonts: &mut FontSelection,
    input: &InputState,
    world: &World,
    screen_width: DVec2,
    mouse: DVec2,
    anim: &AnimationStates,
    draw_calls: usize,
    frame_timer: &WallTimer,
    timers: &BTreeMap<&'static str, Duration>,
) {
    let view = Viewport::new(world.camera, screen_width);
    cmd.current_font_id = world.current_font_id.unwrap();

    if input.is_key_pressed(rdev::Key::Tab) {
        draw_z_index_demo(cmd, &view);
    } else {
        draw_terrain(cmd, world, &view);
    }

    if world.show_detail {
        draw_track_bounds(cmd, world, &view);
        draw_track_chunk_occupancy(cmd, world, &view);
    }
    draw_grid_lines(cmd, &view);
    if !world.show_detail {
        draw_track_junctions(cmd, world, &view);
    }
    draw_tracks(cmd, world, &view, world.show_detail);

    if world.show_detail {
        draw_track_nodes(cmd, world, &view);
    }

    draw_selected_track(cmd, world, sel, &view);
    draw_hovered_track(cmd, world, sel, &view);
    draw_railcars(cmd, world, &view);
    draw_selected_nodes(cmd, world, sel, &view);
    draw_calculated_route(cmd, world, &view);
    draw_hovered_node(cmd, world, sel, &view);
    draw_hovered_chunk(cmd, world, sel, &view);
    draw_clouds(cmd, world, &view);
    draw_ruler(cmd, sel, &view, mouse);

    draw_debug_info(
        cmd,
        world,
        &view,
        sel,
        anim,
        draw_calls,
        timers,
        frame_timer,
    );
    draw_font_ui(cmd, anim, events, fonts, mouse, input);

    cmd.circle(mouse).diameter(11.0).color(Color::RED);
    let mouse_world = view.screen_to_world(mouse);

    {
        let text = format!("{mouse_world:0.2}");
        cmd.text_with_shadow(
            (20.0, 50.0),
            (-2.0, -2.0),
            &text,
            32.0,
            Color::WHITE,
            Color::BLACK.alpha(0.7),
        );
    }
}

fn draw_z_index_demo(cmd: &mut RenderCommands, view: &Viewport) {
    let do_drawing = |cmd: &mut RenderCommands, origin: &mut DVec2, has_z: bool| {
        for z in linspace_f64(0.0, 1.0, 9) {
            let p = *origin + DVec2::new(z * 1000.0, z * 600.0);
            let q = view.world_to_screen(p + DVec2::new(20.0, 250.0));
            let p = view.world_to_screen(p);
            let color = Color::hsl(z * 0.5, 0.7, 0.4, 1.0);
            let d = view.meters(250.0);
            let dims = DVec2::splat(d);
            let mut rect = cmd.rect(p).dims(dims).color(color);
            if has_z {
                rect.z(z);
            } else {
                drop(rect);
            }

            let text = format!(
                "z = {z:0.2}\nhello ifjwefwef\nwow ifjwefwef\nyeee ifjwefwef\nyo ifjwefwef"
            );
            let txt = cmd
                .text(q, text)
                .size(view.meters(32.0))
                .color(Color::WHITE.alpha(0.7));

            if has_z {
                txt.z(z);
            }
        }

        *origin += DVec2::X * 700.0;
    };

    let mut origin = DVec2::new(200.0, 100.0);

    do_drawing(cmd, &mut origin, true);
    do_drawing(cmd, &mut origin, false);
}

fn draw_clouds(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    let alpha = (1.0 - view.zoom() * 10.0).clamp(0.0, 1.0) * 0.4;

    for (pos, radius) in &world.clouds {
        let p = view.world_to_screen_parallax(*pos);
        cmd.circle(p)
            .radius(view.meters(*radius))
            .color(Color::WHITE.alpha(alpha))
            .z(0.1);
    }
}

fn draw_ruler(
    cmd: &mut RenderCommands,
    sel: &SelectionInfo,
    view: &Viewport,
    mouse: DVec2,
) -> Option<()> {
    let ruler_start = sel.ruler_start?;
    let ruler_end = view.screen_to_world(mouse);

    let p = view.world_to_screen(ruler_start);
    let q = view.world_to_screen(ruler_end);
    let d = ruler_start.distance(ruler_end);

    cmd.line(p, q).color(Color::BLACK).thickness(13.0);

    let iso = Isometry2d::new(
        ruler_start.lerp(ruler_end, 0.3),
        (ruler_end - ruler_start).to_angle(),
    );

    let text = distance_str(d);

    cmd.text(view.w2s_iso(iso), text)
        .size(32.0)
        .color(Color::WHITE);

    Some(())
}

fn draw_hovered_chunk(
    cmd: &mut RenderCommands,
    world: &World,
    sel: &SelectionInfo,
    view: &Viewport,
) -> Option<()> {
    let index = sel.hovered_chunk?;
    let id = *world.chunk_map.get(&index)?;
    let chunk = world.chunks.get(id)?;

    let text = format!("{:?}\n{:?}\n{:?}", index, chunk.nodes(), chunk.tracks());

    let size = 32.0f64.max(view.meters(3.0));

    let iso = index.isometry();
    cmd.text(view.w2s_iso(iso), text)
        .size(size)
        .color(Color::WHITE.alpha(0.6));

    let iso = view.w2s_iso(index.isometry());
    let dims = DVec2::splat(view.meters(TERRAIN_CHUNK_WIDTH_METERS));

    cmd.frame(iso, dims).thickness(4.0).color(Color::GRAY);

    Some(())
}

fn draw_calculated_route(cmd: &mut RenderCommands, world: &World, view: &Viewport) -> Option<()> {
    let route = world.calculated_route.as_ref()?;

    for segment_id in route.segments() {
        let track = world.segments.get(*segment_id)?;
        draw_track(cmd, track, world, view, false, Color::PURPLE, 16.0);
    }

    Some(())
}

fn draw_track_bounds(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for track in world.segments.values() {
        let iso = Isometry2d::from_pos(track.lower);
        let mut dims = track.upper - track.lower;
        dims.x = view.meters(dims.x);
        dims.y = view.meters(dims.y);
        let iso = view.w2s_iso(iso);
        cmd.frame(iso, dims)
            .thickness(3.0)
            .color(Color::BLUE.alpha(0.2));
    }
}

fn draw_track_chunk_occupancy(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for track in world.segments.values() {
        for id in &track.chunks {
            let iso = view.w2s_iso(id.isometry());
            let dims = DVec2::splat(view.meters(TERRAIN_CHUNK_WIDTH_METERS));
            cmd.rect(iso).dims(dims).color(Color::BLUE.alpha(0.1));
        }
    }
}

fn draw_railcars(cmd: &mut RenderCommands, world: &World, view: &Viewport) {
    for car in world.cars.values() {
        let Some(track) = world.segments.get(car.segment) else {
            continue;
        };

        let iso = track.eval_at(car.origin, car.pos);

        let color = if car.is_front() {
            Color::PURPLE
        } else {
            Color::BLUE
        };

        draw_railcar(cmd, iso, &view).color(color);
    }
}

fn draw_selected_track(
    cmd: &mut RenderCommands,
    world: &World,
    sel: &SelectionInfo,
    view: &Viewport,
) -> Option<()> {
    let loc = sel.selected_track?;
    let track = world.segments.get(loc.track_id)?;

    draw_track(
        cmd,
        track,
        world,
        view,
        false,
        Color::ORANGE.alpha(0.4),
        32.0,
    );

    let cursor = view.w2s_iso(track.eval_at(loc.origin, loc.pos));

    let cursor = match loc.origin {
        Terminus::Start => cursor,
        Terminus::End => {
            let mut iso = cursor;
            iso.rotation += bary_core::prelude::PI;
            iso.rotation = wrap_0_2pi_f64(iso.rotation as f64) as f32;
            iso
        }
    };

    draw_track_loc_indicator(cmd, cursor, 18.0)
        .thickness(12.0)
        .color(Color::PURPLE.alpha(0.5));

    Some(())
}

fn draw_track_loc_indicator<'a>(
    cmd: &'a mut RenderCommands,
    iso: impl Into<Isometry2d>,
    length: f64,
) -> LineStringBuilder<'a> {
    let iso = iso.into();
    let a = iso.tr();
    let p = iso.offset((-length, length)).tr();
    let q = iso.offset((-length, -length)).tr();

    cmd.linestring(vec![p, a, q])
}

fn draw_hovered_track(
    cmd: &mut RenderCommands,
    world: &World,
    sel: &SelectionInfo,
    view: &Viewport,
) -> Option<()> {
    let HoveredEntity::Track(loc) = sel.hovered? else {
        return None;
    };
    let track = world.segments.get(loc.track_id)?;

    let color = Color::ORANGE.alpha(0.2);
    let p = view.world_to_screen(track.center.translation.as_dvec2());
    let s = view.world_to_screen(track.eval_at(Terminus::Start, 0.0).translation.as_dvec2());

    draw_track(cmd, track, world, &view, false, color, 24.0);
    cmd.circle(s).radii(12.0, 20.0).color(Color::FOREST_GREEN);
    let text = format!(
        "{}\n{:0.1} meters\n{} => {}",
        loc.track_id,
        track.length,
        track.start_node(),
        track.end_node()
    );

    let cursor = view.w2s_iso(track.eval_at(loc.origin, loc.pos));

    let cursor = match loc.origin {
        Terminus::Start => cursor,
        Terminus::End => {
            let mut iso = cursor;
            iso.rotation += bary_core::prelude::PI;
            iso.rotation = wrap_0_2pi_f64(iso.rotation as f64) as f32;
            iso
        }
    };

    draw_track_loc_indicator(cmd, cursor, 18.0)
        .thickness(8.0)
        .color(Color::PURPLE);

    cmd.text(p, text).size(32.0).color(Color::WHITE);

    Some(())
}

fn draw_selected_nodes(
    cmd: &mut RenderCommands,
    world: &World,
    sel: &SelectionInfo,
    view: &Viewport,
) {
    for id in &sel.selected_nodes {
        if let Some(node) = world.nodes.get(*id) {
            cmd.circle(view.world_to_screen(node.pos()))
                .radii(15.0, 25.0)
                .color(Color::ORANGE);
        }
    }
}

fn draw_hovered_node(
    cmd: &mut RenderCommands,
    world: &World,
    sel: &SelectionInfo,
    view: &Viewport,
) -> Option<()> {
    let HoveredEntity::Node(node_id) = sel.hovered? else {
        return None;
    };

    let node = world.nodes.get(node_id)?;

    for track_id in node.linked_tracks() {
        let Some(track) = world.segments.get(*track_id) else {
            continue;
        };
        draw_track(cmd, track, world, view, false, Color::BLUE.alpha(0.7), 12.0);
    }

    let p = view.world_to_screen(node.pos());
    cmd.circle(p).radii(25.0, 35.0).color(Color::PURPLE);
    let text = format!(
        "{:?}\n{:0.1}\n{:?}\n{:?}",
        node_id,
        node.pos(),
        node.forward(),
        node.backward()
    );
    cmd.text(p, text).size(32.0).color(Color::WHITE);

    Some(())
}
