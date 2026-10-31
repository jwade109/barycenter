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
use std::{rc::Rc, time::Duration};

mod backend;
mod mp_backend;
mod sp_backend;

use backend::*;
use mp_backend::*;
use sp_backend::*;

pub struct ClientApp {
    input_handler: InputHandler,
    client: ClientSpecificInfo,
    backend: Box<dyn Backend>,
    terminal: Terminal<TermCmd>,
    assets: Assets,
    ui_state: UiInteractionState,
    update_timer: WallTimer,
    should_exit: bool,
    show_debug_text: bool,

    spawner: EntitySpawner,
    transmission_windows: Components<RadioTransmission>,
}

fn keyboard_control_law(input: &InputState) -> VehicleControl {
    let mut ctrl = VehicleControl::NULLOPT;

    use rdev::Key;

    let docking_mode = input.is_key_pressed(Key::ControlLeft);

    if docking_mode {
        ctrl.plus_x.throttle = input.is_key_pressed(Key::UpArrow) as u8 as f32;
        ctrl.plus_y.throttle = input.is_key_pressed(Key::RightArrow) as u8 as f32;
        ctrl.neg_x.throttle = input.is_key_pressed(Key::DownArrow) as u8 as f32;
        ctrl.neg_y.throttle = input.is_key_pressed(Key::LeftArrow) as u8 as f32;
    } else {
        ctrl.plus_x.throttle = input.is_key_pressed(Key::UpArrow) as u8 as f32;
        ctrl.neg_x.throttle = input.is_key_pressed(Key::DownArrow) as u8 as f32;

        ctrl.attitude = if input.is_key_pressed(Key::LeftArrow) {
            10.0
        } else if input.is_key_pressed(Key::RightArrow) {
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

impl ClientApp {
    fn new(
        args: Args,
        log_level: log::LevelFilter,
    ) -> Result<(Self, Raylib), Box<dyn std::error::Error>> {
        let input_handler = InputHandler::new();

        let world = World::empty();

        let mut terminal = Terminal::new();

        terminal.register_commands(all_commands());
        terminal.register_commands(client_request_blob_commands());
        terminal.register_commands(world_delta_commands());
        terminal.register_commands(terminal_commands());

        _ = simple_logger::SimpleLogger::new()
            .with_level(log_level)
            .env()
            .init();

        let (mut handle, thread) = raylib::init()
            .size(1080, 700)
            .title("Barycenter")
            .log_level(TraceLogLevel::LOG_WARNING)
            .msaa_4x()
            .resizable()
            .build();

        handle.set_target_fps(1000);
        handle.maximize_window();
        handle.set_exit_key(None);
        handle.hide_cursor();

        let mut raylib = Raylib::new(handle, thread);

        let mut assets = Assets::default();
        load_assets(&mut assets, &mut raylib);

        let update_timer = WallTimer::with_dur(Duration::from_millis(20));
        let ui_state = UiInteractionState::default();

        let backend: Box<dyn Backend> = if args.use_mp_backend {
            let node = ClientNode::with_str_addr(&args.server_addr)?;
            Box::new(MultiplayerBackend::new(world, node, args.username))
        } else {
            Box::new(SinglePlayerBackend::new(world))
        };

        let app = ClientApp {
            backend,
            client: ClientSpecificInfo::new(),
            input_handler,
            terminal,
            assets,
            ui_state,
            update_timer,
            should_exit: false,
            show_debug_text: false,
            spawner: EntitySpawner::default(),
            transmission_windows: Components::default(),
        };

        Ok((app, raylib))
    }

    fn update_in_world(&mut self) {
        let deltas = pre_simulation_update(self.backend.world(), &mut self.client);

        let ctrl = keyboard_control_law(&self.client.input);

        let ship_control = self
            .client
            .scene_graph
            .free()
            .and_then(|f| f.follow_vehicle.map(|id| (id, ctrl)));

        let controls = ExternalControls { ship_control };

        for msg in self.backend.update(&controls) {
            self.on_rcv_server_msg(msg);
        }

        let post_deltas = post_simulation_update(
            self.backend.world(),
            &mut self.client,
            self.terminal.is_focused(),
        );

        for delta in deltas.into_iter().chain(post_deltas) {
            self.backend.request_delta(delta);
        }

        let cmds = self.terminal.handle_input(&self.client.input);

        for cmd in cmds {
            self.on_terminal_cmd(cmd);
        }

        let on_gui = self.ui_state.is_on_gui();
        if !self.terminal.is_active() {
            process_event(self.backend.world(), &mut self.client, on_gui);

            if let Some(editor) = self.client.scene_graph.editor_mut() {
                if let Some(delta) = editor.handle_keys(&self.client.input, self.backend.world()) {
                    self.backend.request_delta(delta);
                }
            }
        }
    }

    fn update(&mut self, raylib: &Raylib) {
        // GET SOME BASIC INPUT INFORMATION FROM RAYLIB

        self.client.mouse_screen_position = raylib
            .handle()
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(raylib.handle().get_mouse_position()));

        self.client.screen_dims = Vec2::new(
            raylib.handle().get_screen_width() as f32,
            raylib.handle().get_screen_height() as f32,
        );

        self.client.input = self
            .input_handler
            .update(raylib.handle().is_window_focused());

        match &self.client.scene_graph {
            _ => self.update_in_world(),
        }
    }

    pub fn on_rcv_server_msg(&mut self, msg: ServerMessage) {
        let s = format!("{:?}", msg);
        self.terminal.log_debug(s);

        debug!("{msg:?}");

        match msg {
            ServerMessage::ChatMessage(id, text) => {
                let s = format!("[{id}]: {text}");
                self.client.chat.log(s);
            }
            ServerMessage::TransactionComplete => {
                self.client.scene_graph = SceneGraph::Free(FreeFlying::new());
            }
            ServerMessage::SyncFrame(frame) => {
                debug!("Got frame: {:?}", frame);
                let our_frame = sync_frame_from_world(self.backend.world());
                debug!("Our frame: {:?}", our_frame);
            }
            ServerMessage::PlayerConnected(username) => {
                self.client.chat.log(format!("{username} just connected."));
            }
            ServerMessage::PlayerDisconnected(username) => {
                self.client
                    .chat
                    .log(format!("{username} just disconnected."));
            }
            ServerMessage::ServerStatistics(_) => (),
            ServerMessage::Simulation(sim) => {
                self.on_sim_event(sim);
            }
            _ => error!("Unsupported message from server: {:?}", msg),
        }
    }

    fn on_sim_event(&mut self, sim: SimulationEvent) {
        let id = self.spawner.spawn();
        self.transmission_windows.spawn(id, sim.transmission);
    }

    fn exit(&mut self) {
        info!("Exiting cleanly.");
        self.should_exit = true;
    }

    fn docking_rotate(&mut self) {
        if let Some(free) = self.client.scene_graph.free_mut() {
            free.rotation = free.rotation.next();
        }
    }

    fn docking_shift(&mut self, delta: Vec2, is_x: bool) {
        if let Some(free) = self.client.scene_graph.free_mut() {
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
        if let Some(free) = self.client.scene_graph.free()
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
        self.client.sounds.push(SoundEffect::ButtonDown);

        let Some(onclick) = ui.onclick else {
            return;
        };

        match onclick {
            UiMessage::Navigate(nav) => {
                self.client.scene_graph.handle_navigation(nav);
            }
            UiMessage::Exit => self.exit(),
            UiMessage::AltMode => self.client.alt_mode ^= true,
            UiMessage::DebugText => self.show_debug_text ^= true,
            UiMessage::SimSpeed(sp) => self.backend.set_sim_speed(sp),
            UiMessage::DockingRotate => self.docking_rotate(),
            UiMessage::DockingActivate => self.docking_activate(),
            UiMessage::LoadSaveFile(path) => self.load_save_file(path),
            UiMessage::LoadSave => {
                self.client.scene_graph = SceneGraph::ListSaves;
            }
            UiMessage::JoinMultiplayer => {
                self.backend.request_world_download();
            }
            UiMessage::WriteSaveToDisk => {
                save_world_and_alert_chat(self.backend.world(), &mut self.client);
            }
            UiMessage::Delta(delta) => {
                self.backend.request_delta(delta);
            }
            UiMessage::Editor(cmd) => {
                if let SceneGraph::Editor(state) = &mut self.client.scene_graph {
                    state.handle_command(cmd)
                }
            }
            UiMessage::AckTransmission(id) => {
                _ = self.transmission_windows.despawn(id);
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
                self.client.sounds.push(SoundEffect::ButtonUp);
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

fn make_gui(
    raylib: &Raylib,
    font: Rc<Font>,
    app: &mut ClientApp,
    timers: &DebugTimers,
) -> Tree<UiMessage> {
    let mut ui = UiBuilder::new(
        font,
        app.client.input.clone(),
        std::mem::take(&mut app.ui_state),
    );

    match &mut app.client.scene_graph {
        SceneGraph::MainMenu => {
            main_menu_ui(&mut ui);
        }
        SceneGraph::Free(free) => {
            make_free_flying_ui(&mut ui, free, app.backend.world(), &app.assets);
            make_part_info_gui(&mut ui, app.backend.world(), &app.client, &app.assets);
            make_transmission_ui(&mut ui, &app.transmission_windows);
        }
        SceneGraph::Editor(editor) => {
            make_editor_ui(&mut ui, editor, app.backend.world());
        }
        SceneGraph::Part(state) => {
            part_info_modal(&mut ui, state, app.backend.world(), &app.assets);
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

    if app.show_debug_text {
        make_debug_ui(&mut ui, app, timers, raylib.handle().get_fps());
    }

    let (tree, state) = ui.take();

    app.ui_state = state;

    tree
}

fn make_debug_ui(ui: &mut UiBuilder, app: &ClientApp, timers: &DebugTimers, fps: u32) {
    debug_ui(
        ui,
        app.backend.world(),
        &app.client,
        timers,
        app.backend.network(),
        &app.update_timer,
        app.backend.username(),
        app.client.player_id,
        fps,
    )
}

fn server_thread(args: Args) {
    if !args.run_server {
        return;
    }

    _ = simple_logger::init_with_level(Level::Info);

    info!("Starting server in background thread");

    let mut server = match BaryServer::new(args.server_port, 10) {
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

fn draw(mut d: &mut RaylibDrawHandle, app: &ClientApp, ui: Tree<UiMessage>) {
    d.clear_background(Color::BLACK);

    if matches!(
        app.client.scene_graph,
        SceneGraph::Editor(..) | SceneGraph::Free(..) | SceneGraph::Part(..)
    ) {
        draw_world(app.backend.world(), &app.client, &app.assets, &mut d);
    }

    draw_ui(&mut d, &app.ui_state, &ui, &app.assets);

    draw_mouse_screen_position(&mut d, app.client.mouse_screen_position);

    draw_terminal(&mut d, &app.terminal, &app.assets, Color::BLACK.alpha(0.8));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let server_args = args.clone();

    _ = std::thread::Builder::new()
        .name("server_background_thread".into())
        .spawn(|| server_thread(server_args))?;

    info!("{:?}", args);

    let sounds = SoundHandler::new();

    let (mut main_app, mut raylib) = ClientApp::new(args, log::LevelFilter::Info)?;

    while !raylib.handle().window_should_close() && !main_app.should_exit {
        if !main_app.update_timer.tick() {
            continue;
        }

        main_app.update(&raylib);

        // AND DRAW IT ALL

        let font = main_app.assets.ui_font();

        let timers = DebugTimers::default();

        let ui = make_gui(&raylib, font, &mut main_app, &timers);

        if let Some(c) = main_app.ui_state.update(
            &ui,
            main_app.client.mouse_screen_position,
            &main_app.client.input,
        ) {
            main_app.on_ui_event(c);
        }

        raylib.draw(|mut d: RaylibDrawHandle<'_>| {
            draw(&mut d, &main_app, ui.clone());
        });

        sounds.play_sounds(std::mem::take(&mut main_app.client.sounds));

        if main_app.client.input.just_pressed(rdev::Key::KeyC)
            && main_app.client.input.is_key_pressed(rdev::Key::ControlLeft)
        {
            break;
        }
    }

    info!("Done.");

    Ok(())
}
