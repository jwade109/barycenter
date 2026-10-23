use crate::*;
use bary_core::prelude::*;
use bary_ipc::ClientNode;
use bary_sim::*;

pub fn debug_lines(
    world: &World,
    client: &ClientSpecificInfo,
    timers: &DebugTimers,
    node: &ClientNode,
    update_timer: &WallTimer,
    ping_timer: &WallTimer,
    tlm_timer: &WallTimer,
    username: &str,
    user_id: Option<Ent>,
    fps: u32,
) -> Vec<String> {
    let size = size_in_bytes(world);

    let mut ret = Vec::new();

    ret.push("Barycenter Client".to_string());

    let consist = is_world_consistent(world);
    ret.push(format!("OK: {:?}", consist));

    let fmt_time = |d: std::time::Duration, t: std::time::Duration| {
        let p = d.as_secs_f64() / t.as_secs_f64();
        format!(
            "    {:09} ns\n    {:05.04} ms\n    {:3.1}%",
            d.as_nanos(),
            d.as_nanos() as f64 / 1000000.0,
            p * 100.0
        )
    };

    ret.push(format!(
        " {}/{} {} {:0.1} {}",
        timers.ticks,
        client.tick_rate,
        world.ticks,
        apparent_elapsed_time(world.ticks).as_secs_f64(),
        apparent_datetime(world.ticks).format("%b %d %Y %I:%M:%S %p"),
    ));

    ret.push(format!("Username:  {}", username));
    if let Some(uid) = user_id {
        ret.push(format!("User ID:   {}", uid));
    }

    ret.push(format!("FPS:       {}", fps));
    ret.push(format!("Memory:    {:0.3} KB", size as f64 / 1000.0));
    ret.push(format!("Zoom:      {:0.3}", client.camera.zoom));

    ret.push(format!(
        "Update:    {:0.2} Hz / {:0.2} Hz ",
        update_timer.actual_rate(),
        update_timer.nominal_rate()
    ));
    ret.push(format!(
        "Ping:      {:0.2} Hz / {:0.2} Hz ",
        ping_timer.actual_rate(),
        ping_timer.nominal_rate()
    ));
    ret.push(format!(
        "TLM:       {:0.2} Hz / {:0.2} Hz ",
        tlm_timer.actual_rate(),
        tlm_timer.nominal_rate()
    ));

    ret.push("".into());

    ret.push(format!("Connected: {}", node.is_connected()));
    ret.push(format!("Address:   {}", node.server_addr()));
    ret.push(format!("RX:        {}", node.rx_count()));
    ret.push(format!("TX:        {}", node.tx_count()));
    ret.push(format!("Errors:    {}", node.errors()));

    ret.push("".into());

    ret.push(format!("Ticks:     {}", world.ticks));
    ret.push(format!("Grids:     {}", world.grids.len()));
    ret.push(format!("Parts:     {}", world.parts.len()));
    ret.push(format!("Protos:    {}", world.prototypes.len()));
    ret.push(format!("BPs:       {}", world.blueprints.len()));
    ret.push(format!("Thrusters: {}", world.thrusters.len()));
    ret.push(format!("Invs:      {}", world.inventories.len()));
    ret.push(format!("Machines:  {}", world.machines.len()));
    ret.push(format!("Asteroids: {}", world.asteroids.len()));
    ret.push(format!("Chunks:    {}", world.terrain_chunks.len()));
    ret.push(format!("Tiles:     {}", world.terrain_tiles.len()));
    ret.push(format!("GAUs:      {}", world.grid_acceleration_updates));
    ret.push(format!("Players:   {}", world.players.len()));

    for player in world.players.values() {
        ret.push(format!("  {} {}", player.name, player.state));
    }

    ret.push("".into());

    let total = timers.total();

    ret.push(format!("total{}", fmt_time(total, total)));

    for timer in timers.timers.iter() {
        let time = fmt_time(*timer.1, total);
        ret.push(format!("{}{}", timer.0, time));
    }

    ret
}

pub fn debug_ui(
    ui: &mut UiBuilder,
    world: &World,
    client: &ClientSpecificInfo,
    timers: &DebugTimers,
    node: &ClientNode,
    update_timer: &WallTimer,
    ping_timer: &WallTimer,
    tlm_timer: &WallTimer,
    username: &str,
    user_id: Option<Ent>,
    fps: u32,
) {
    // let old_font_size = ui.font_size;
    // ui.font_size = 9.0;
    ui.begin_window("debug_ui");
    for line in debug_lines(
        world,
        client,
        timers,
        node,
        update_timer,
        ping_timer,
        tlm_timer,
        username,
        user_id,
        fps,
    ) {
        ui.text(line);
    }

    ui.end_window();
    // ui.font_size = old_font_size;
}
