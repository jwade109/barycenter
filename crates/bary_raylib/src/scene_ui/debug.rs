use crate::*;
use bary_core::prelude::*;
use bary_ipc::ClientNode;
use bary_sim::*;

pub fn debug_lines(
    ui: &mut UiBuilder,
    world: &World,
    client: &ClientSpecificInfo,
    timers: &DebugTimers,
    network: Option<&ClientNode>,
    update_timer: &WallTimer,
    username: Option<&str>,
    user_id: Option<Ent>,
    fps: u32,
) {
    ui.window("debug_ui", |ui| {
        let size = size_in_bytes(world);

        ui.text("Barycenter Client".to_string());

        let consist = is_world_consistent(world);
        ui.text(format!("OK: {:?}", consist));

        let fmt_time = |d: std::time::Duration, t: std::time::Duration| {
            let p = d.as_secs_f64() / t.as_secs_f64();
            format!(
                "    {:09} ns\n    {:05.04} ms\n    {:3.1}%",
                d.as_nanos(),
                d.as_nanos() as f64 / 1000000.0,
                p * 100.0
            )
        };

        ui.text(format!(
            " {}/{} {} {:0.1} {}",
            timers.ticks,
            client.tick_rate,
            world.ticks,
            apparent_elapsed_time(world.ticks).as_secs_f64(),
            apparent_datetime(world.ticks).format("%b %d %Y %I:%M:%S %p"),
        ));

        if let Some(username) = username {
            ui.text(format!("Username:  {}", username));
        }

        if let Some(uid) = user_id {
            ui.text(format!("User ID:   {}", uid));
        }

        ui.text(format!("FPS:       {}", fps));
        ui.text(format!("Memory:    {:0.3} KB", size as f64 / 1000.0));
        ui.text(format!("Zoom:      {:0.3}", client.camera.zoom));

        ui.text(format!(
            "Update:    {:0.2} Hz / {:0.2} Hz ",
            update_timer.actual_rate(),
            update_timer.nominal_rate()
        ));

        ui.sep();

        if let Some(network) = network {
            ui.text(format!("Connected: {}", network.is_connected()));
            ui.text(format!("Address:   {}", network.server_addr()));
            ui.text(format!("RX:        {}", network.rx_count()));
            ui.text(format!("TX:        {}", network.tx_count()));
            ui.text(format!("Errors:    {}", network.errors()));
            ui.sep();
        }

        ui.text(format!("Ticks:     {}", world.ticks));
        ui.text(format!("Grids:     {}", world.grids.len()));
        ui.text(format!("Parts:     {}", world.parts.len()));
        ui.text(format!("Protos:    {}", world.prototypes.len()));
        ui.text(format!("BPs:       {}", world.blueprints.len()));
        ui.text(format!("Thrusters: {}", world.thrusters.len()));
        ui.text(format!("Invs:      {}", world.inventories.len()));
        ui.text(format!("Machines:  {}", world.machines.len()));
        ui.text(format!("Asteroids: {}", world.asteroids.len()));
        ui.text(format!("Chunks:    {}", world.terrain_chunks.len()));
        ui.text(format!("Tiles:     {}", world.terrain_tiles.len()));
        ui.text(format!("GAUs:      {}", world.grid_acceleration_updates));
        ui.text(format!("Players:   {}", world.players.len()));

        ui.sep();
        ui.text(format!("{:?}", ui.hot_element()));
        ui.text(format!("{:?}", ui.active_element()));
        ui.sep();

        for player in world.players.values() {
            ui.text(format!("  {} {}", player.name, player.state));
        }

        ui.text("");

        let total = timers.total();

        ui.text(format!("total{}", fmt_time(total, total)));

        for timer in timers.timers.iter() {
            let time = fmt_time(*timer.1, total);
            ui.text(format!("{}{}", timer.0, time));
        }
    });
}

pub fn debug_ui(
    ui: &mut UiBuilder,
    world: &World,
    client: &ClientSpecificInfo,
    timers: &DebugTimers,
    network: Option<&ClientNode>,
    update_timer: &WallTimer,
    username: Option<&str>,
    user_id: Option<Ent>,
    fps: u32,
) {
    debug_lines(
        ui,
        world,
        client,
        timers,
        network,
        update_timer,
        username,
        user_id,
        fps,
    );
}
