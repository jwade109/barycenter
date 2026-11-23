use bary_core::prelude::*;
use bary_input::{InputHandler, InputState};
use bary_ipc::*;
use bary_orbital::VehicleControl;
use bary_raylib::*;
use bary_server::*;
use bary_sim::*;
use bary_terminal::Terminal;
use bary_ui::*;
use clap::Parser;
use log::*;
use raylib::prelude::*;
use rdev::{Button, Key};
use std::time::Duration;

mod backend;
mod mp_backend;
mod sp_backend;

use backend::*;
use mp_backend::*;
use sp_backend::*;

pub struct GameApp {
    input_handler: InputHandler,
    frontend: FrontEnd,
    backend: Box<dyn Backend>,
    terminal: Terminal<TermCmd>,
    ui_state: UiInteractionState,
    should_exit: bool,
}

#[derive(Debug, Clone, Copy)]
struct PilotingKeybinds {
    left: Key,
    right: Key,
    forward: Key,
    backward: Key,
    mode: Button,
}

fn keyboard_control_law(input: &InputState, keybinds: PilotingKeybinds) -> VehicleControl {
    let mut ctrl = VehicleControl::NULLOPT;

    let docking_mode = input.is_key_pressed(keybinds.mode);

    if docking_mode {
        ctrl.plus_x.throttle = input.is_key_pressed(keybinds.forward) as u8 as f32;
        ctrl.plus_y.throttle = input.is_key_pressed(keybinds.right) as u8 as f32;
        ctrl.neg_x.throttle = input.is_key_pressed(keybinds.backward) as u8 as f32;
        ctrl.neg_y.throttle = input.is_key_pressed(keybinds.left) as u8 as f32;
    } else {
        ctrl.plus_x.throttle = input.is_key_pressed(keybinds.forward) as u8 as f32;
        ctrl.neg_x.throttle = input.is_key_pressed(keybinds.backward) as u8 as f32;

        ctrl.attitude = if input.is_key_pressed(keybinds.left) {
            10.0
        } else if input.is_key_pressed(keybinds.right) {
            -10.0
        } else {
            0.0
        };
    }

    ctrl.plus_x.use_rcs = docking_mode;
    ctrl.plus_y.use_rcs = docking_mode;
    ctrl.neg_x.use_rcs = docking_mode;
    ctrl.neg_y.use_rcs = docking_mode;

    ctrl
}

fn make_terminal() -> Terminal<TermCmd> {
    let mut terminal = Terminal::new();

    terminal.register_commands(all_commands());
    terminal.register_commands(client_request_blob_commands());
    terminal.register_commands(world_delta_commands());
    terminal.register_commands(terminal_commands());

    terminal
}

impl GameApp {
    fn new(
        args: Args,
        log_level: log::LevelFilter,
    ) -> Result<(Self, Raylib), Box<dyn std::error::Error>> {
        let input_handler = InputHandler::new();

        let world = World::empty();

        _ = simple_logger::SimpleLogger::new()
            .with_level(log_level)
            .env()
            .init();

        let mut raylib = Raylib::init();

        let mut assets = Assets::default();
        load_assets(&mut assets, &mut raylib);

        let ui_state = UiInteractionState::default();

        let username = args.username.clone();

        let backend: Box<dyn Backend> = if args.use_mp_backend {
            let node = ClientNode::with_str_addr(&args.server_addr)?;
            Box::new(MultiplayerBackend::new(world, node, args.username))
        } else {
            Box::new(SinglePlayerBackend::new(world))
        };

        let app = GameApp {
            backend,
            frontend: FrontEnd::new(username, assets),
            input_handler,
            terminal: make_terminal(),
            ui_state,
            should_exit: false,
        };

        Ok((app, raylib))
    }

    fn update_in_world(&mut self) {
        let deltas = pre_simulation_update(self.backend.world(), &mut self.frontend);

        for event in self.backend.update() {
            self.on_rcv_backend_event(event);
        }

        self.frontend.player_id = self.backend.allocated_player_id();

        let post_deltas = post_simulation_update(
            self.backend.world(),
            &mut self.frontend,
            self.terminal.is_focused(),
        );

        for delta in deltas.take().into_iter().chain(post_deltas.take()) {
            self.backend.request_delta(delta);
        }

        let cmds = self.terminal.handle_input(&self.frontend.input);

        for cmd in cmds {
            self.on_terminal_cmd(cmd);
        }

        let on_gui = self.ui_state.is_on_gui();
        if !self.terminal.is_active() {
            process_event(self.backend.world(), &mut self.frontend, on_gui);

            if let Some(editor) = self.frontend.scene_graph.editor_mut() {
                if let Some(delta) = editor.handle_keys(&self.frontend.input, self.backend.world())
                {
                    self.backend.request_delta(delta);
                }
            }
        }
    }

    fn update(&mut self, raylib: &Raylib) {
        // GET SOME BASIC INPUT INFORMATION FROM RAYLIB

        self.frontend.keybinds.clear_active();
        self.frontend.keybinds.set_active("global");

        match &self.frontend.scene_graph {
            SceneGraph::MainMenu => {
                self.frontend.keybinds.set_active("menu");
            }
            SceneGraph::ListSaves => {
                self.frontend.keybinds.set_active("menu");
            }
            SceneGraph::Kicked => {
                self.frontend.keybinds.set_active("menu");
            }
            SceneGraph::Settings => {
                self.frontend.keybinds.set_active("menu");
            }
            SceneGraph::Editor(..) => {
                self.frontend.keybinds.set_active("editor");
            }
            SceneGraph::Free(..) => {
                self.frontend.keybinds.set_active("freecamera");
            }
            SceneGraph::LockedToPlayer(..) => {
                self.frontend.keybinds.set_active("locked_to_player");
            }
        }

        self.frontend.mouse_screen_position = raylib
            .handle()
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(raylib.handle().get_mouse_position()));

        self.frontend.screen_dims = Vec2::new(
            raylib.handle().get_screen_width() as f32,
            raylib.handle().get_screen_height() as f32,
        );

        self.frontend.input = self
            .input_handler
            .update(raylib.handle().is_window_focused());

        match &self.frontend.scene_graph {
            _ => self.update_in_world(),
        }

        self.frontend.update();

        if self.frontend.input_timer.tick() {
            let mut ctrl = PlayerControls::default();

            if self.frontend.is_satisfied("player_controller::left") {
                ctrl.x = JoystickPosition::Negative;
            }
            if self.frontend.is_satisfied("player_controller::right") {
                ctrl.x = JoystickPosition::Positive;
            }
            if self.frontend.is_satisfied("player_controller::down") {
                ctrl.y = JoystickPosition::Negative;
            }
            if self.frontend.is_satisfied("player_controller::up") {
                ctrl.y = JoystickPosition::Positive;
            }
            if self.frontend.is_satisfied("player_controller::rotate_ccw") {
                ctrl.a = JoystickPosition::Positive;
            }
            if self.frontend.is_satisfied("player_controller::rotate_cw") {
                ctrl.a = JoystickPosition::Negative;
            }

            if let Some(id) = self.backend.allocated_player_id() {
                let delta = WorldDelta::SetPlayerControls(id, ctrl);
                self.backend.request_delta(delta);
            }
        }
    }

    pub fn on_rcv_backend_event(&mut self, event: BackendEvent) {
        match event {
            BackendEvent::FinishedWorldDownload(name) => {
                let message = format!("Finished world download \"{name}\"");
                self.frontend.spawn_animation(message, true);
                self.frontend.scene_graph = SceneGraph::Free(FreeFlying::at_zero());
            }
            BackendEvent::Server(response) => {
                self.on_rcv_server_msg(response);
            }
        }
    }

    pub fn on_rcv_server_msg(&mut self, msg: ServerResponse) {
        let s = format!("{:?}", msg);
        self.terminal.log_debug(s);

        debug!("{msg:?}");

        match msg {
            ServerResponse::ChatMessage(id, text) => {
                let s = format!("[{id}]: {text}");
                self.frontend.chat.log(s);
                self.frontend.spawn_animation(text, false);
            }
            ServerResponse::SyncFrame(frame) => {
                debug!("Got frame: {:?}", frame);
                let our_frame = sync_frame_from_world(self.backend.world());
                debug!("Our frame: {:?}", our_frame);
            }
            ServerResponse::PlayerConnected(username) => {
                self.frontend
                    .chat
                    .log(format!("{username} just connected."));
            }
            ServerResponse::PlayerDisconnected(username) => {
                self.frontend
                    .chat
                    .log(format!("{username} just disconnected."));
            }
            ServerResponse::Simulation(sim) => {
                self.on_sim_event(sim);
            }
            ServerResponse::Driver { deltas, .. } => {
                for delta in deltas {
                    if !matches!(delta, WorldDelta::SetPlayerControls(..)) {
                        let s = format!("{:?}", delta);
                        self.frontend.spawn_animation(s, true);
                    }
                }
            }
            ServerResponse::ClientConnected(id, addr) => {
                let msg = format!("Client connected: {id}, {addr}");
                self.frontend.spawn_animation(msg, true);
            }
            ServerResponse::ClientDisconnected(id, addr) => {
                let msg = format!("Client disconnected: {id}, {addr}");
                self.frontend.spawn_animation(msg, true);
            }
            ServerResponse::NewSimSpeed(speed) => {
                let msg = if speed == 0 {
                    "Sim paused".to_string()
                } else {
                    format!("New sim speed: {speed}")
                };
                self.frontend.spawn_animation(msg, true);
            }
            ServerResponse::Telemetry(_) => {
                // PASS
            }
            ServerResponse::ClientTelemetry(id, tlm) => {
                // info!("Got TLM from client {id}: {tlm:?}");
            }
            _ => error!("Unsupported message from server: {:?}", msg),
        }
    }

    fn on_sim_event(&mut self, sim: SimulationEvent) {
        let id = self.frontend.spawner.spawn();
        info!("Got sim event: {sim:?}");
        match sim {
            SimulationEvent::Transmission(rt) => {
                let msg = format!("TR: {} {:?}", rt.received_by, rt.original.contents());
                self.frontend.transmissions.spawn(id, rt);
                self.frontend.spawn_animation(msg, true)
            }
            SimulationEvent::Piloting(id, username) => {
                info!("{username} is piloting {id}");
            }
        }
    }

    fn exit(&mut self) {
        info!("Exiting cleanly.");
        self.should_exit = true;
    }

    fn docking_rotate(&mut self) {
        if let Some(free) = self.frontend.scene_graph.free_mut() {
            free.rotation = free.rotation.next();
        }
    }

    fn docking_shift(&mut self, delta: Vec2, is_x: bool) {
        if let Some(free) = self.frontend.scene_graph.free_mut() {
            if is_x {
                let delta = delta.with_y(0.0) / 4.0;
                let off = PartCoord(vround(delta));
                free.offset += off;
            } else {
                let delta = Vec2::new(0.0, delta.x);
                let off = PartCoord(vround(delta));
                free.offset += off;
            }
        }
    }

    fn docking_activate(&mut self) {
        if let Some(free) = self.frontend.scene_graph.free()
            && let Some(docking) = free.docking_interface()
        {
            let delta = WorldDelta::MergeGrids(
                docking.parent.grid_id,
                docking.child.grid_id,
                docking.offset,
                docking.rotation,
            );
            self.backend.request_delta(delta);
        }
    }

    fn load_save_file(&mut self, path: String) {
        info!("Commanding server to load save file");
        self.backend.load_save(path);
    }

    pub fn on_drag(&mut self, id: UiRef, delta: Vec2) {
        debug!("Drag event: {:?}", id);
        match id.onclick {
            Some(UiMessage::DockingShiftX) => {
                self.docking_shift(delta, true);
            }
            Some(UiMessage::DockingShiftY) => {
                self.docking_shift(delta, false);
            }
            Some(UiMessage::Drag(handle)) => {
                self.ui_state.shift_offset(handle, delta);
            }
            Some(e) => warn!("Unhandled drag event: {e:?}"),
            None => (),
        }
    }

    pub fn on_click(&mut self, ui: UiRef) {
        info!("Click event: {:?}", ui);
        self.frontend.sounds.push(SoundEffect::ButtonDown);

        let Some(onclick) = ui.onclick else {
            return;
        };

        match onclick {
            UiMessage::Navigate(nav) => {
                self.frontend.scene_graph.handle_navigation(nav);
            }
            UiMessage::Exit => self.exit(),
            UiMessage::AltMode => self.frontend.toggle_alt_mode(),
            UiMessage::DebugText => self.frontend.toggle_debug_text(),
            UiMessage::SimSpeed(sp) => self.backend.set_sim_speed(sp),
            UiMessage::DockingRotate => self.docking_rotate(),
            UiMessage::DockingActivate => self.docking_activate(),
            UiMessage::LoadSaveFile(path) => self.load_save_file(path),
            UiMessage::LoadSave => {
                self.frontend.scene_graph = SceneGraph::ListSaves;
            }
            UiMessage::JoinMultiplayer => {
                self.backend.request_world_download();
            }
            UiMessage::WriteSaveToDisk => {
                save_world_and_alert_chat(self.backend.world(), &mut self.frontend);
            }
            UiMessage::Delta(delta) => {
                self.backend.request_delta(delta);
            }
            UiMessage::Editor(cmd) => {
                if let SceneGraph::Editor(state) = &mut self.frontend.scene_graph {
                    state.handle_command(cmd)
                }
            }
            UiMessage::AckTransmission(id) => {
                _ = self.frontend.transmissions.despawn(id);
            }
            UiMessage::ClearTransmissions => {
                self.frontend.transmissions.clear();
            }
            e => warn!("Unhandled click event: {e:?}"),
        }
    }

    pub fn on_ui_event(&mut self, c: UiEvent) {
        match c.kind {
            UiEventKind::Drag(delta) => {
                self.on_drag(c.reference, delta);
            }
            UiEventKind::Release => {
                self.frontend.sounds.push(SoundEffect::ButtonUp);
            }
            UiEventKind::Click => self.on_click(c.reference),
        }
    }

    pub fn on_terminal_cmd(&mut self, cmd: TermCmd) {
        self.terminal.log_info(format!("{cmd:?}"));

        match cmd {
            TermCmd::Clear => {
                self.terminal.clear();
            }
            TermCmd::SetSimSpeed(speed) => {
                self.backend.set_sim_speed(speed);
            }
            TermCmd::Exit => {
                self.exit();
            }
            TermCmd::ClientReqAllBlobs => {
                self.terminal.log_warn(format!(
                    "Requesting all blobs at tick {}",
                    self.backend.world().ticks
                ));
                self.backend.request_world_download();
            }
            TermCmd::World(delta) => {
                self.backend.request_delta(delta);
            }
            TermCmd::Help => {
                self.terminal.print_help_command();
            }
            TermCmd::ToggleDebugMode => {
                self.terminal.toggle_debug_mode();
            }
            _ => self.terminal.log_error(format!("Unsupported: {:?}", cmd)),
        }
    }
}

/// Run the test client app
#[derive(Parser, Debug, Default, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long)]
    username: String,
    #[arg(short = 'a', default_value = "127.0.0.1:5000")]
    server_addr: String,
    #[arg(short = 'p', default_value = "5000")]
    server_port: u16,
    #[arg(long)]
    run_server: bool,
    #[arg(long)]
    use_mp_backend: bool,
}

fn make_gui(raylib: &Raylib, app: &mut GameApp, timers: &DebugTimers) -> Tree<UiMessage> {
    let mut ui = UiBuilder::new(
        app.frontend.assets.ui_font(),
        app.frontend.input.clone(),
        std::mem::take(&mut app.ui_state),
    );

    match &mut app.frontend.scene_graph {
        SceneGraph::MainMenu => {
            main_menu_ui(&mut ui);
        }
        SceneGraph::Free(free) => {
            make_free_flying_ui(&mut ui, free, app.backend.world(), &app.frontend.assets);
            make_part_info_gui(&mut ui, app.backend.world(), &app.frontend);
            // TODO
            app.frontend.transmissions.clear();
            make_transmission_ui(&mut ui, &app.frontend.transmissions);
        }
        SceneGraph::LockedToPlayer(locked) => {
            kicked_scene_ui(&mut ui);
            // make_piloting_ui(&mut ui, piloting, app.backend.world(), &app.frontend.assets);
        }
        SceneGraph::Editor(editor) => {
            make_editor_ui(&mut ui, editor, app.backend.world());
        }
        SceneGraph::ListSaves => {
            list_saves_scene_ui(&mut ui);
        }
        SceneGraph::Kicked => {
            kicked_scene_ui(&mut ui);
        }
        SceneGraph::Settings => {
            settings_scene_ui(&mut ui);
        }
    }

    if app.frontend.show_debug_text {
        make_debug_ui(&mut ui, app, timers, raylib.handle().get_fps());
    }

    if app.frontend.show_keybinds {
        make_keybinds_ui(&mut ui, app);
    }

    let (tree, state) = ui.take();

    app.ui_state = state;

    tree
}

fn make_keybinds_ui(ui: &mut UiBuilder, app: &GameApp) {
    ui.window("keybinds", |ui| {
        for gid in app.frontend.keybinds.active_groups() {
            ui.text(gid);
        }

        ui.sep();

        for (id, kb) in app.frontend.keybinds.iter() {
            let is_on = app.frontend.is_satisfied(id);
            let prefix = if is_on { "##" } else { "  " };
            let color = if is_on { Color::TEAL } else { Color::WHITE };
            let s = format!("{prefix} {:50} {}", id, kb);
            ui.colored_text(s, color);
        }
    });
}

fn make_debug_ui(ui: &mut UiBuilder, app: &GameApp, timers: &DebugTimers, fps: u32) {
    debug_ui(
        ui,
        app.backend.world(),
        &app.frontend,
        timers,
        app.backend.network(),
        app.backend.username(),
        app.frontend.player_id,
        fps,
    )
}

fn server_thread(args: Args) {
    if !args.run_server {
        return;
    }

    _ = simple_logger::init_with_level(Level::Info);

    info!("Starting server in background thread");

    let mut server = match BaryServer::new(args.server_port) {
        Ok(server) => server,
        Err(e) => {
            error!("Failed to start server: {e:?}");
            return;
        }
    };

    loop {
        server.update();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn draw_animations(d: &mut RaylibDrawHandle, animations: &Components<AppearingText>) {
    for anim in animations.values() {
        anim.draw(d);
    }
}

fn draw(d: &mut RaylibDrawHandle, app: &GameApp, ui: Tree<UiMessage>) {
    d.clear_background(Color::BLACK);

    if let Some(blob) = &app.frontend.assets.blob {
        d.draw_texture_ex(
            blob,
            Vector2::new(100.0, 100.0),
            0.0,
            0.5,
            Color::WHITE.alpha(0.3),
        );
    }

    if app.frontend.scene_graph.should_draw_world() {
        draw_world(app.backend.world(), &app.frontend, d);
    }

    draw_ui(d, &app.ui_state, &ui, &app.frontend.assets);

    draw_animations(d, &app.frontend.animations);

    draw_mouse_screen_position(d, app.frontend.mouse_screen_position);

    draw_terminal(
        d,
        &app.terminal,
        &app.frontend.assets,
        Color::BLACK.alpha(0.8),
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let server_args = args.clone();

    _ = std::thread::Builder::new()
        .name("server_background_thread".into())
        .spawn(|| server_thread(server_args))?;

    info!("{:?}", args);

    let sounds = SoundHandler::new();

    let (mut app, mut raylib) = GameApp::new(args, log::LevelFilter::Info)?;

    while !raylib.handle().window_should_close() && !app.should_exit {
        std::thread::sleep(Duration::from_millis(10));

        app.update(&raylib);

        // AND DRAW IT ALL

        let timers = DebugTimers::default();

        let ui = make_gui(&raylib, &mut app, &timers);

        if let Some(c) =
            app.ui_state
                .update(&ui, app.frontend.mouse_screen_position, &app.frontend.input)
        {
            app.on_ui_event(c);
        }

        raylib.draw(|mut d: RaylibDrawHandle<'_>| {
            draw(&mut d, &app, ui.clone());
        });

        sounds.play_sounds(std::mem::take(&mut app.frontend.sounds));

        if app.frontend.is_satisfied("global::exit_app") {
            break;
        }

        if app.frontend.is_satisfied("global::toggle_show_debug_text") {
            app.frontend.toggle_debug_text();
        }

        if app.frontend.is_satisfied("global::toggle_show_keybinds") {
            app.frontend.toggle_show_keybinds();
        }

        if app.frontend.is_satisfied("global::toggle_alt_mode") {
            app.frontend.toggle_alt_mode();
        }

        if app.frontend.is_satisfied("global::save_world") {
            save_world_and_alert_chat(app.backend.world(), &mut app.frontend);
        }

        if app.frontend.is_satisfied("global::toggle_camera_lock") {
            app.frontend.toggle_camera_lock(app.backend.world());
        }
    }

    info!("Done.");

    Ok(())
}
