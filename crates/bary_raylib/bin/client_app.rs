use bary_core::prelude::*;
use bary_factory::*;
use bary_input::{InputHandler, InputState};
use bary_ipc::*;
use bary_raylib::*;
use bary_server::*;
use bary_sim::*;
use bary_terminal::Terminal;
use bary_ui::*;
use clap::Parser;
use early_returns::*;
use log::*;
use raylib::prelude::*;
use std::time::Duration;

enum AppState {
    Loading(u32, u32, String),
    MainMenu,
    ListSaves,
    InWorld,
    Kicked,
    Settings,
}

pub struct ClientApp {
    state: AppState,
    input: InputHandler,

    username: String,

    client: ClientSpecificInfo,
    world: World,

    terminal: Terminal<TermCmd>,

    handle: RaylibHandle,
    thread: RaylibThread,
    assets: Assets,
    node: ClientNode,
    ui_state: UiInteractionState,

    update_timer: WallTimer,
    server_ping_timer: WallTimer,
    server_telemetry_timer: WallTimer,

    should_exit: bool,

    show_debug_text: bool,

    sounds: SoundEffects,

    menu_origin: Vec2,
    part_info_origin: Vec2,
    server_handshake_complete: bool,

    async_world_download: Option<AsyncWorldDownload>,
}

impl ClientApp {
    fn new(args: Args, log_level: log::LevelFilter) -> Result<Self, Box<dyn std::error::Error>> {
        let input = InputHandler::new();

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

        let mut assets = Assets::default();
        load_assets(&mut assets, &mut handle, &thread);

        let node = ClientNode::with_str_addr(&args.server_addr)?;

        let update_timer = WallTimer::with_dur(Duration::from_millis(20));
        let server_ping_timer = WallTimer::with_dur(Duration::from_millis(1000));
        let server_telemetry_timer = WallTimer::with_dur(Duration::from_millis(100));
        let ui_state = UiInteractionState::default();

        Ok(ClientApp {
            state: AppState::ListSaves,
            username: args.username,
            world,
            client: ClientSpecificInfo::new(),
            input,
            terminal,
            handle,
            thread,
            assets,
            node,
            ui_state,
            update_timer,
            server_ping_timer,
            server_telemetry_timer,
            should_exit: false,
            show_debug_text: false,
            sounds: SoundEffects::new(),
            menu_origin: Vec2::new(300.0, 200.0),
            part_info_origin: Vec2::new(400.0, 350.0),
            server_handshake_complete: false,
            async_world_download: None,
        })
    }

    fn request_server_delta(&mut self, delta: WorldDelta) {
        self.node.send(MessageKind::RequestDelta(delta));
    }

    fn send_telemetry_to_server(&mut self) {
        let tlm = ClientTelemetry {
            ticks: self.world.ticks,
        };
        self.node.send(MessageKind::ClientTelemetry(tlm));
    }

    fn update_loading(&mut self) {
        if let AppState::Loading(steps, total, status) = &mut self.state {
            if chance(0.8) && *steps < *total {
                *steps += randint(4, 12) as u32;
                *status = get_random_name();
            }

            if *steps >= *total {
                self.state = AppState::MainMenu;
            }
        }
    }

    fn update_in_world(&mut self) {
        let deltas = pre_simulation_update(&self.world, &mut self.client);

        for msg in self.node.update() {
            self.on_rcv_server_msg(msg);
        }

        let post_deltas = post_simulation_update(
            &self.world,
            &mut self.client,
            &mut self.sounds,
            self.terminal.is_focused(),
        );

        for delta in deltas.into_iter().chain(post_deltas) {
            self.node.send(MessageKind::RequestDelta(delta));
        }

        // HANDLE INPUTS FROM RDEV LISTENER THREAD

        if self.server_ping_timer.tick() {
            if self.server_handshake_complete {
                self.node.send(MessageKind::Ping)
            }
        }

        if self.server_telemetry_timer.tick() {
            if self.server_handshake_complete {
                self.send_telemetry_to_server();
            }
        }

        // HANDLE RDEV EVENTS (DEPRECATED - USE INPUTSTATE)

        let cmds = self.terminal.handle_input(&self.client.input);

        for cmd in cmds {
            self.on_terminal_cmd(cmd);
        }

        let on_gui = self.ui_state.is_on_gui();
        if !self.terminal.is_active() {
            process_event(&mut self.world, &mut self.client, &mut self.sounds, on_gui);

            if let Some(editor) = self.client.viewport.editor_mut() {
                let zoom = self.client.camera.zoom;
                if let Some(delta) = editor.handle_keys(&self.client.input, &self.world, zoom) {
                    self.node.send(MessageKind::RequestDelta(delta));
                }
            }
        }
    }

    fn update(&mut self) {
        // GET SOME BASIC INPUT INFORMATION FROM RAYLIB

        self.client.mouse_screen_position = self
            .handle
            .is_cursor_on_screen()
            .then(|| raylib_to_glam(self.handle.get_mouse_position()));

        self.client.screen_dims = Vec2::new(
            self.handle.get_screen_width() as f32,
            self.handle.get_screen_height() as f32,
        );

        for event in self.input.get_events() {
            let focused = self.handle.is_window_focused();
            self.client.input.process_rdev_event(&event, focused);
        }

        match &self.state {
            AppState::Loading(_, _, _) => self.update_loading(),
            _ => self.update_in_world(),
        }
    }

    pub fn on_rcv_server_msg(&mut self, msg: Message) {
        let s = format!("{:?}", msg);
        self.terminal.log_debug(s);

        debug!("{msg:?}");

        match msg.kind {
            MessageKind::ChatMessage(id, text) => {
                let s = format!("[{id}]: {text}");
                self.client.chat.log(s);
            }
            MessageKind::Driver {
                ticks,
                deltas,
                players,
            } => {
                self.on_driver_packet(ticks, deltas);
                self.world.players = players;
            }
            MessageKind::BlobResponse(blob) => {
                self.on_rcv_blob(blob);
            }
            MessageKind::MultiBlobResponse(ent, ticks, blobs) => {
                let delta = ticks as i64 - self.world.ticks as i64;
                self.terminal.log_warn(format!(
                    "Delta ticks: {} AKA {:0.3}",
                    delta,
                    timedelta_from_delta_ticks(delta).as_seconds_f64()
                ));
                self.world.spawner.set_next(ent);
                self.world.ticks = ticks;
                for blob in blobs {
                    self.on_rcv_blob(blob);
                }
            }
            MessageKind::FinishWorldDownload => {
                self.state = AppState::InWorld;
            }
            MessageKind::SyncFrame(frame) => {
                debug!("Got frame: {:?}", frame);
                let our_frame = sync_frame_from_world(&self.world);
                debug!("Our frame: {:?}", our_frame);
            }
            MessageKind::HaveNewSave => {
                self.on_rcv_have_new_save();
            }
            MessageKind::Kicked => {
                self.on_rcv_kicked();
            }
            MessageKind::WhoGoesThere => {
                self.on_rcv_who_goes_there();
            }
            MessageKind::TakeYourHatOff => {
                self.on_rcv_take_your_hat_off();
            }
            MessageKind::PlayerConnected(username) => {
                self.client.chat.log(format!("{username} just connected."));
            }
            MessageKind::PlayerDisconnected(username) => {
                self.client
                    .chat
                    .log(format!("{username} just disconnected."));
            }
            MessageKind::Ack => (),
            MessageKind::Pong => (),
            MessageKind::ServerStatistics(_) => (),
            _ => error!("Unsupported message from server: {:?}", msg.kind),
        }
    }

    fn on_rcv_take_your_hat_off(&mut self) {
        self.server_handshake_complete = true;
    }

    fn on_rcv_who_goes_there(&mut self) {
        self.node.send(MessageKind::Introduction {
            username: self.username.clone(),
        });
    }

    fn on_rcv_kicked(&mut self) {
        self.node.disconnect();
        self.state = AppState::Kicked;
        self.world = World::empty();
    }

    fn on_rcv_have_new_save(&mut self) {
        self.node.send(MessageKind::BeginAsyncWorldDownload);
        self.async_world_download = Some(AsyncWorldDownload::new());
    }

    fn log_world_delta(&mut self, delta: &WorldDelta) {
        match delta {
            WorldDelta::SetPlayerPosition(_, _) => return,
            WorldDelta::SetPlayerCursorPosition(_, _) => return,
            _ => {
                self.client.chat.log(format!("{:?}", delta));
            }
        }
    }

    fn on_driver_packet(&mut self, ticks: u64, deltas: Vec<WorldDelta>) {
        while self.world.ticks + 1 < ticks {
            update_world(&mut self.world);
        }

        for delta in deltas {
            self.log_world_delta(&delta);
            if let Err(e) = apply_delta(&mut self.world, delta.clone()) {
                error!("Failed to apply delta {:?}: {:?}", delta, e);
            }
        }

        if self.world.ticks < ticks {
            update_world(&mut self.world);
        }
    }

    fn on_rcv_blob(&mut self, blob: Blob) {
        if let Some(dl) = &mut self.async_world_download {
            dl.add_blob(blob);

            if dl.is_complete() {
                let dl = self
                    .async_world_download
                    .take()
                    .expect("Expected this to be Some(_)");

                self.world = dl.world();
            }
        } else {
            error!("No download underway!");
        }
    }

    fn exit(&mut self) {
        info!("Exiting cleanly.");
        self.should_exit = true;
    }

    fn save(&mut self) {
        save_world_and_alert_chat(&self.world, &mut self.client);
    }

    fn open_editor(&mut self) {
        enter_ship_editor(&self.world, &mut self.client, &mut self.sounds)
    }

    fn leave_editor(&mut self) {
        if self.client.leave_editor() {
            self.sounds.push(SoundEffect::LeaveEditor);
        }
    }

    fn set_sim_speed(&mut self, speed: u32) {
        self.node.send(MessageKind::SetSimSpeed(speed));
    }

    fn docking_rotate(&mut self) {
        if let Some(free) = self.client.viewport.free_mut() {
            free.rotation = free.rotation.next();
        }
    }

    fn docking_shift(&mut self, delta: Vec2, is_x: bool) {
        if let Some(free) = self.client.viewport.free_mut() {
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
        if let Some(docking) = self.client.docking_interface() {
            let delta = WorldDelta::MergeGrids(
                docking.parent.grid_id,
                docking.child.grid_id,
                docking.offset,
                docking.rotation,
            );
            self.node.send(MessageKind::RequestDelta(delta));
        }
    }

    fn load_save_file(&mut self, path: String) {
        info!("Commanding server to load save file");
        self.node.send(MessageKind::LoadSave(path));
    }

    pub fn on_drag(&mut self, id: UiMessage, delta: Vec2) {
        match id {
            UiMessage::DockingShiftX => {
                self.docking_shift(delta, true);
            }
            UiMessage::DockingShiftY => {
                self.docking_shift(delta, false);
            }
            UiMessage::DockingHandle => {
                self.menu_origin += delta;
            }
            UiMessage::MainMenuHandle => {
                self.menu_origin += delta;
            }
            UiMessage::SaveSelectHandle => {
                self.menu_origin += delta;
            }
            UiMessage::PartInfoHandle => {
                self.part_info_origin += delta;
            }
            e => warn!("Unhandled event: {e:?}"),
        }
    }

    pub fn on_ui_event(&mut self, c: UiEvent) {
        debug!("{:?}", c);

        match c.kind {
            UiEventKind::Drag(delta) => {
                self.on_drag(c.msg, delta);
            }
            UiEventKind::Release => {
                self.sounds.push(SoundEffect::ButtonUp);
            }
            UiEventKind::Click => {
                self.sounds.push(SoundEffect::ButtonDown);
                match c.msg {
                    UiMessage::Exit => self.exit(),
                    UiMessage::SaveFile => self.save(),
                    UiMessage::OpenEditor => self.open_editor(),
                    UiMessage::LeaveEditor => self.leave_editor(),
                    UiMessage::AltMode => self.client.alt_mode ^= true,
                    UiMessage::DebugText => self.show_debug_text ^= true,
                    UiMessage::SimSpeed(sp) => self.set_sim_speed(sp),
                    UiMessage::DockingRotate => self.docking_rotate(),
                    UiMessage::DockingActivate => self.docking_activate(),
                    UiMessage::LoadSaveFile(path) => self.load_save_file(path),
                    UiMessage::GoToMainMenu => {
                        self.state = AppState::MainMenu;
                    }
                    UiMessage::SetMachineState(machine_id, state) => {
                        let delta = WorldDelta::SetMachineState(machine_id, state);
                        self.request_server_delta(delta);
                    }
                    UiMessage::SetThrusterState(thruster_id, state) => {
                        let delta = WorldDelta::SetThrusterState(thruster_id, state);
                        self.request_server_delta(delta);
                    }
                    UiMessage::LoadSave => {
                        self.state = AppState::ListSaves;
                    }
                    UiMessage::JoinMultiplayer => {
                        self.request_world_download();
                    }
                    UiMessage::Settings => {
                        self.state = AppState::Settings;
                    }
                    e => warn!("Unhandled event: {e:?}"),
                }
            }
        }
    }

    fn request_world_download(&mut self) {
        self.node.send(MessageKind::BeginAsyncWorldDownload);
        self.async_world_download = Some(AsyncWorldDownload::new());
    }

    pub fn on_terminal_cmd(&mut self, cmd: TermCmd) {
        self.terminal.log_info(format!("{cmd:?}"));

        match cmd {
            TermCmd::Say(s) => {
                self.node.send(MessageKind::ClientSays(s));
            }
            TermCmd::Ping => {
                self.node.send(MessageKind::Ping);
            }
            TermCmd::Clear => {
                self.terminal.clear();
            }
            TermCmd::SetSimSpeed(speed) => {
                self.set_sim_speed(speed);
            }
            TermCmd::FindGridByName(name) => {
                self.node.send(MessageKind::FindGridByName(name));
            }
            TermCmd::Exit => {
                self.exit();
            }
            TermCmd::PrintEntityInfo(table) => {
                self.node.send(MessageKind::PrintEntityInfo(table));
            }
            TermCmd::ServerConnect => {
                self.node.reconnect();
            }
            TermCmd::ServerDisconnect => {
                self.node.disconnect();
            }
            TermCmd::RequestServerStatistics => {
                self.node.send(MessageKind::RequestServerStatistics);
            }
            TermCmd::ClientReqBlob(table) => {
                self.node.send(MessageKind::ClientBlobRequest(table));
            }
            TermCmd::ClientReqAllBlobs => {
                self.terminal
                    .log_warn(format!("Requesting all blobs at tick {}", self.world.ticks));
                self.request_world_download();
            }
            TermCmd::World(delta) => {
                self.node.send(MessageKind::RequestDelta(delta));
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
    #[arg(short)]
    save_file: String,
    #[arg(short = 'a', default_value = "127.0.0.1:5000")]
    server_addr: String,
    #[arg(short = 'p', default_value = "5000")]
    server_port: u16,
    #[arg(long)]
    run_server: bool,
}

#[allow(unused)]
fn generate_assets_ui(assets: &Assets, input: &InputState, width: f32, height: f32) -> Tree<()> {
    let mut root = Node::new(width, Size::Fit);

    for (ctx, mapping) in assets.keybinds.iter() {
        let mut col = Node::new(Size::Grow, height / 2.0)
            .down()
            .with_padding(4.0)
            .with_child_gap(1.0);

        let s = format!("{:?}", ctx);
        let header = Node::text(Size::Grow, 40, s);

        col.add_child(header);

        let actions = ActionSet::new(input, mapping);

        for (key, action) in mapping.iter() {
            let color = if actions.just_triggered(*action) {
                Color::TEAL
            } else if actions.is_active(*action) {
                Color::new(60, 60, 110, 255)
            } else {
                Color::new(20, 20, 20, 255)
            };

            let color = [
                color.r as f32 / 255.0,
                color.g as f32 / 255.0,
                color.b as f32 / 255.0,
                color.a as f32 / 255.0,
            ];

            let line = format!("{:?}: {:?}", key, action);
            let row = Node::text(Size::Grow, 30, line).with_color(color);
            col.add_child(row);
        }

        root.add_child(col);

        // x += width + padding;
    }

    Tree::new().with_layout(root, None)
}

pub struct UiBuilder<'a> {
    font: &'a Font,
    font_size: f32,
}

impl<'a> UiBuilder<'a> {
    fn new(font: &'a Font) -> Self {
        Self {
            font,
            font_size: UI_FONT_SIZE as f32,
        }
    }

    fn draghandle<T: UiMsg>(&self, msg: impl Into<T>) -> Node<T> {
        Node::<T>::handle(Size::Grow, 20, msg)
    }

    fn sep<T: UiMsg>(&self) -> Node<T> {
        Node::structural(Size::Grow, 2)
    }

    fn text<T: UiMsg>(&self, s: impl Into<String>) -> Node<T> {
        let s = s.into();
        let dims = self.font.measure_text(&s, self.font_size, 0.0);
        let w = dims.x;
        let h = dims.y;
        Node::text(w, h, s)
    }

    fn button<T: UiMsg>(&self, msg: impl Into<T>, s: impl Into<String>) -> Node<T> {
        let s = s.into();
        let dims = self.font.measure_text(&s, self.font_size, 0.0);
        let w = dims.x + 36.0;
        let h = dims.y + 18.0;
        Node::button(s, msg, w, h)
    }

    fn sprite<T: UiMsg>(&self, s: impl Into<String>, texture: &Texture2D) -> Node<T> {
        let h = 70;
        let actual_w = texture.width;
        let actual_h = texture.height;
        let scale = actual_h as f32 / h as f32;
        let w = actual_w as f32 / scale;
        Node::image(s, w as u32, h)
    }

    fn progress_bar<T: UiMsg>(&self, val: f32) -> Node<T> {
        Node::progress_bar(Size::Grow, 11, val)
    }
}

fn make_docking_ui(
    builder: &UiBuilder,
    docking: &DockingInterface,
    world: &World,
) -> Option<Node<UiMessage>> {
    let parent = world.grids.try_get(docking.parent.grid_id).ok()?;
    let child = world.grids.try_get(docking.child.grid_id).ok()?;

    let sep = parent
        .particle_location
        .translation
        .distance(child.particle_location.translation);

    let sep = distance_str(sep as f64);

    let docking_ui = Node::column(
        Size::Fit,
        vec![
            builder.draghandle(UiMessage::DockingHandle),
            builder.text("Docking Control Panel"),
            builder.text("Multiline string\nAnother line\nAnd another!"),
            builder.text(format!("{} <- {}", parent.name, child.name)),
            builder.text(format!("Separation: {}", sep)),
            builder.button(UiMessage::DockingShiftX, "Offset X"),
            builder.button(UiMessage::DockingShiftY, "Offset Y"),
            builder.button(UiMessage::DockingRotate, "Rotate"),
            builder.button(UiMessage::DockingActivate, "Dock"),
        ],
    );

    Some(docking_ui)
}

fn make_main_menu(ui: &UiBuilder) -> Node<UiMessage> {
    Node::column(
        Size::Fit,
        vec![
            ui.draghandle(UiMessage::MainMenuHandle),
            ui.button(UiMessage::LoadSave, "Load Single Player"),
            ui.button(UiMessage::JoinMultiplayer, "Join Server"),
            // ui.button(UiMessage::JoinMultiplayer, "Join Multiplayer"),
            // ui.button(UiMessage::HostMultiplayer, "Host Multiplayer"),
            ui.button(UiMessage::Settings, "Settings"),
            ui.button(UiMessage::Exit, "Exit to Desktop"),
        ],
    )
}

fn loading_layout(ui: &UiBuilder, steps: u32, total: u32, status: &str) -> Node<UiMessage> {
    let progress = steps as f32 / total as f32;
    let pstr = format!("{}/{}", steps, total);
    Node::column(
        500,
        vec![
            ui.text("Loading..."),
            ui.progress_bar(progress),
            ui.text(pstr),
            ui.text(status),
        ],
    )
}

fn make_savefiles_menu(ui: &UiBuilder, saves: &[String]) -> Node<UiMessage> {
    let mut buttons = vec![ui.draghandle(UiMessage::SaveSelectHandle)];

    for s in saves {
        let b = ui.button(UiMessage::LoadSaveFile(s.clone()), s.clone());
        buttons.push(b);
    }

    buttons.push(ui.button(UiMessage::GoToMainMenu, "Return to Main Menu"));

    Node::column(Size::Fit, buttons)
}

fn make_world_ui(ui: &UiBuilder, app: &ClientApp, tree: &mut Tree<UiMessage>) {
    let mut root: Node<UiMessage> = Node::root(Size::Fit, Size::Fit).with_children(
        [
            ui.button(UiMessage::GoToMainMenu, "Return to Main Menu"),
            ui.button(UiMessage::SaveFile, "Save Game"),
            ui.button(UiMessage::AltMode, "Toggle Alt Mode"),
            ui.button(UiMessage::DebugText, "Toggle Debug Text"),
            ui.button(UiMessage::SimSpeed(0), "Toggle Pause"),
            ui.button(UiMessage::SimSpeed(1), "Sim 1x"),
            ui.button(UiMessage::SimSpeed(10), "Sim 10x"),
            ui.button(UiMessage::SimSpeed(100), "Sim 100x"),
            ui.button(UiMessage::SimSpeed(1000), "Sim 1000x"),
        ]
        .into_iter(),
    );

    if app.client.selected_grid_loc().is_some() && app.client.viewport.editor().is_none() {
        let b = ui.button(UiMessage::OpenEditor, "Open Ship Editor");
        root.add_child(b);
    }

    if app.client.viewport.editor().is_some() {
        root.add_child(ui.button(UiMessage::LeaveEditor, "Leave Ship Editor"));
    }

    tree.add_layout(root, None);

    if let Some(docking) = app.client.docking_interface() {
        if let Some(ui) = make_docking_ui(ui, &docking, &app.world) {
            tree.add_layout(ui, app.menu_origin)
        }
    }
}

fn slot_info_str(idx: usize, slot: &InvSlot) -> String {
    if let Some(contents) = slot.contents() {
        format!(
            "{}. {:?} ({}) ({:0.1}%) {}",
            idx + 1,
            contents.0,
            contents.1,
            100.0 * slot.fill_percentage(),
            slot.mass(),
        )
    } else {
        format!("{}. Empty - can store [{:?}]", idx + 1, slot.filter())
    }
}

fn computer_info_str(cpu: &Computer) -> String {
    let mut lines = vec![
        format!("On: {}", cpu.on),
        format!("\nStatus: {:?}", cpu.status),
        format!("\nTicks: {}", cpu.ticks_this_cycle),
        format!("\nFired: {}", cpu.fired_this_tick),
        format!("\nIters: {}", cpu.iters),
    ];

    for cmd in &cpu.command_queue {
        let line = format!("\n  - {}", cmd);
        lines.push(line);
    }

    lines.into_iter().collect()
}

fn make_part_info_gui(ui: &UiBuilder, app: &ClientApp, tree: &mut Tree<UiMessage>) {
    let gridloc = some_or_return!(app.client.selected_grid_loc());
    let grid = ok_or_return!(app.world.grids.try_get(gridloc.grid_id));
    let occ = some_or_return!(grid.get_parts_at(gridloc.coord));

    let mut children = Vec::new();

    let s = format!(
        "At {}-{}: {:?}",
        gridloc.grid_id,
        gridloc.coord,
        occ.to_array()
    );

    children.push(ui.draghandle(UiMessage::PartInfoHandle));
    children.push(ui.text(s));

    for (layer, part_id) in occ.iter() {
        children.push(ui.sep());

        let part = ok_or_continue!(app.world.parts.try_get(part_id));
        let part_local = part.region.to_local(gridloc.coord);
        let proto = ok_or_continue!(app.world.prototypes.try_get(part.prototype));

        children.push(ui.text(proto.name.clone()));

        if let Some(texture) = app.assets.part_textures.get(&proto.name) {
            children.push(ui.sprite(proto.name.clone(), texture));
        }

        if let Ok(cpu) = app.world.computers.try_get(part_id) {
            children.push(ui.button(
                UiMessage::SetComputerOnOff(part_id, !cpu.on),
                if cpu.on { "Turn Off" } else { "Turn On" },
            ));
            children.push(ui.button(UiMessage::SetComputerDrift(part_id), "Set to Drift"));
            let info = computer_info_str(cpu);
            children.push(ui.text(info));
        }
        if let Ok(thruster) = app.world.thrusters.try_get(part_id) {
            let s = format!("{:#?}", thruster);
            children.push(ui.button(
                UiMessage::SetThrusterState(part_id, !thruster.is_on()),
                if thruster.is_on() {
                    "Turn Off"
                } else {
                    "Turn On"
                },
            ));
            children.push(ui.text(s));
        }
        if let Ok(light) = app.world.lights.try_get(part_id) {
            let s = format!("{:#?}", light);
            children.push(ui.text(s));
        }
        if let Ok(mac) = app.world.machines.try_get(part_id) {
            children.push(ui.progress_bar(mac.progress()));
            children.push(ui.button(
                UiMessage::SetMachineState(part_id, !mac.enabled),
                if mac.enabled { "Turn Off" } else { "Turn On" },
            ));
        }
        if let Ok(inv) = app.world.inventories.try_get(part_id) {
            for (idx, slot) in inv.slots().enumerate() {
                let line = slot_info_str(idx, slot);
                children.push(ui.text(line));
                children.push(ui.progress_bar(slot.fill_percentage()));
            }
        }

        if app.client.alt_mode {
            let mut s = format!("Part ID: {}", part_id);
            s += &format!("\nPart local coord: {}", part_local);

            s += &format!(
                "\nRegion: {:?} {} {:?}",
                layer,
                part.region.bottom_left(),
                part.region.rot()
            );

            s += &format!(
                "\nPrototype: {} {} {:?}",
                proto.name,
                proto.mass,
                proto.classification()
            );
            children.push(ui.text(s));
        }
    }

    let node = Node::column(600, children);
    tree.add_layout(node, app.part_info_origin);
}

fn make_gui(font: &Font, app: &ClientApp) -> Tree<UiMessage> {
    let builder = UiBuilder::new(font);

    let mut tree = Tree::new();

    match &app.state {
        AppState::Loading(steps, total, status) => {
            tree.add_layout(
                loading_layout(&builder, *steps, *total, status),
                app.menu_origin,
            );
        }
        AppState::MainMenu => {
            tree.add_layout(make_main_menu(&builder), app.menu_origin);
        }
        AppState::InWorld => {
            make_world_ui(&builder, app, &mut tree);
            make_part_info_gui(&builder, &app, &mut tree);
        }
        AppState::ListSaves => {
            let mut saves = list_saves_in_dir("saves/");
            saves.sort();
            let saves: Vec<_> = saves
                .into_iter()
                .map(|s| s.to_string_lossy().to_string())
                .collect();
            tree.add_layout(make_savefiles_menu(&builder, &saves), app.menu_origin);
        }
        AppState::Kicked => {
            tree.add_layout(
                Node::text(600, 100, "You've been kicked."),
                Vec2::splat(200.0),
            );
        }
        AppState::Settings => {
            tree.add_layout(Node::text(600, 100, "The settings."), Vec2::splat(200.0));
        }
    }

    if let Some(dl) = &app.async_world_download {
        let node = loading_layout(&builder, dl.steps(), dl.total(), dl.status());
        tree.add_layout(node, Vec2::new(400.0, 200.0));
    }

    tree
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

    server.load_world(args.save_file);

    loop {
        server.update();
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let server_args = args.clone();

    std::thread::spawn(move || server_thread(server_args));

    info!("{:?}", args);

    let sounds = SoundHandler::new();

    let mut main_app = ClientApp::new(args, log::LevelFilter::Info)?;

    while !main_app.handle.window_should_close() && !main_app.should_exit {
        if !main_app.update_timer.tick() {
            continue;
        }

        main_app.update();

        // AND DRAW IT ALL

        let font = main_app.assets.ui_font();

        let ui = make_gui(font, &main_app);

        let scrp = main_app.client.mouse_screen_position;

        if let Some(c) = main_app.ui_state.update(&ui, scrp, &main_app.client.input) {
            main_app.on_ui_event(c);
        }

        let timers = DebugTimers::default();

        main_app
            .handle
            .draw(&main_app.thread, |mut d: RaylibDrawHandle<'_>| {
                d.clear_background(Color::BLACK);

                if matches!(main_app.state, AppState::InWorld) {
                    draw_world(&main_app.world, &main_app.client, &main_app.assets, &mut d);
                }

                draw_gui(&mut d, &main_app.ui_state, &ui, &main_app.assets);

                lame_old_imgui_entrypoint(
                    &mut d,
                    &mut main_app.client,
                    &main_app.world,
                    &mut main_app.sounds,
                );

                draw_mouse_screen_position(&mut d, main_app.client.mouse_screen_position);

                if main_app.show_debug_text {
                    draw_debug_info(
                        &main_app.world,
                        &main_app.client,
                        &main_app.assets,
                        &timers,
                        &main_app.node,
                        &main_app.update_timer,
                        &main_app.server_ping_timer,
                        &main_app.server_telemetry_timer,
                        &main_app.username,
                        main_app.client.player_id,
                        &mut d,
                    );
                }

                draw_terminal(
                    &mut d,
                    &main_app.terminal,
                    &main_app.assets,
                    Color::BLACK.alpha(0.8),
                );
            });

        sounds.play_sounds(main_app.sounds.drain(..).collect());

        if main_app.client.input.just_pressed(rdev::Key::KeyC)
            && main_app.client.input.is_key_pressed(rdev::Key::ControlLeft)
        {
            break;
        }

        main_app.client.input.on_frame_boundary();
    }

    info!("Done.");

    Ok(())
}
