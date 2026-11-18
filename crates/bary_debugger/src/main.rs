use bary_core::prelude::*;
use bary_ipc::{ClientNode, GridTelemetry, ServerRequest, ServerResponse};
use bary_server::WorldDownloader;
use bary_sim::{Sender, World, WorldDelta, apply_delta, update_world};
use eframe::egui;
use egui::{Id, Ui, WidgetText};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use log::{Level, error, info, warn};
use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc;
use std::time::Duration;

mod comparator;
mod dialogue;
mod grid_commander;
mod plotter;

use crate::comparator::WorldComparator;
use crate::dialogue::DialogueSender;
use crate::grid_commander::VehicleDetailWidget;
use crate::plotter::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Tab {
    PositionPlot,
    HeadingPlot,
    ThrustersPlot,
    Buttons,
    VehicleDetailWidget,
    Comparator,
    Dialogue,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tab::HeadingPlot => "Heading",
            Tab::PositionPlot => "Position",
            Tab::ThrustersPlot => "Thrusters",
            Tab::Buttons => "Buttons",
            Tab::VehicleDetailWidget => "Grid Commander",
            Tab::Comparator => "Comparator",
            Tab::Dialogue => "Dialogue",
        };
        f.write_str(s)
    }
}

impl TabViewer for App {
    // This associated type is used to attach some data to each tab.
    type Tab = Tab;

    // Returns the unique ID for the `tab`.
    fn id(&mut self, tab: &mut Self::Tab) -> Id {
        Id::new(tab)
    }

    // Returns the current `tab`'s title.
    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.to_string().into()
    }

    // Defines the contents of a given `tab`.
    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let mut deltas = Sender::new();

        match tab {
            Tab::HeadingPlot => {
                self.heading.draw(ui);
            }
            Tab::PositionPlot => {
                self.pos.draw(ui);
            }
            Tab::ThrustersPlot => {
                self.thrusters.draw(ui);
            }
            Tab::Buttons => {
                self.button_panel(ui);
            }
            Tab::VehicleDetailWidget => {
                self.commander.draw(ui, &self.grid_info, &mut deltas);
            }
            Tab::Comparator => {
                self.comparator.ui(ui);
            }
            Tab::Dialogue => {
                self.dialogue.ui(ui, &mut self.sender);
            }
        }

        for delta in deltas.take() {
            _ = self.sender.send(delta.into());
        }
    }
}
struct App {
    sender: mpsc::Sender<ServerRequest>,
    receiver: mpsc::Receiver<ServerResponse>,
    telemetry: BTreeMap<u8, ServerResponse>,
    logs: VecDeque<ServerResponse>,

    grid_info: BTreeMap<Ent, GridTelemetry>,

    server_ticks: u64,
    n_clients: usize,

    pos: Plotter,
    heading: Plotter,
    thrusters: Plotter,
    commander: VehicleDetailWidget,

    download: Option<WorldDownloader>,
    world: World,
    comparator: WorldComparator,
    dialogue: DialogueSender,

    dock_state: DockState<Tab>,
}

fn default_layout() -> DockState<Tab> {
    let mut dock_state = DockState::new(vec![Tab::PositionPlot, Tab::Comparator, Tab::Dialogue]);

    // Currently, the `DockState` only has one `Surface`: the main one.
    // Let's get mutable access to add more nodes in it.
    let surface = dock_state.main_surface_mut();

    // Insert "tab2" to the left of "tab1", where the width of "tab2"
    // is 20% of root node's width.
    let [old_node, new_node] =
        surface.split_left(NodeIndex::root(), 0.15, vec![Tab::VehicleDetailWidget]);

    surface.split_below(new_node, 0.5, vec![Tab::Buttons]);

    surface.split_below(old_node, 0.5, vec![Tab::HeadingPlot, Tab::ThrustersPlot]);

    dock_state
}

impl Default for App {
    fn default() -> Self {
        let client = ClientNode::localhost(5000);

        let (request_sender, request_receiver) = mpsc::channel();
        let (response_sender, response_receiver) = mpsc::channel();

        std::thread::spawn(|| {
            background_thread(client, request_receiver, response_sender);
        });

        let dialogue = DialogueSender::new("assets/dialogue.yaml");

        Self {
            sender: request_sender,
            receiver: response_receiver,
            telemetry: BTreeMap::new(),
            logs: VecDeque::new(),
            server_ticks: 0,
            n_clients: 0,
            pos: Plotter::new("position", 1.0),
            heading: Plotter::new("heading", None),
            thrusters: Plotter::new("thrusters", None),
            commander: VehicleDetailWidget::new(),
            dock_state: default_layout(),
            grid_info: BTreeMap::new(),
            world: World::empty(),
            download: None,
            comparator: WorldComparator::new(),
            dialogue,
        }
    }
}

fn background_thread(
    mut client: ClientNode,
    request_receiver: mpsc::Receiver<ServerRequest>,
    response_sender: mpsc::Sender<ServerResponse>,
) {
    client.send(ServerRequest::Ping);

    loop {
        for msg in client.update() {
            response_sender.send(msg).unwrap();
        }

        for request in request_receiver.try_iter() {
            client.send(request);
        }

        std::thread::sleep(Duration::from_millis(5));
    }
}

impl App {
    fn button_panel(&mut self, ui: &mut egui::Ui) {
        if ui.button("Clear Plots").clicked() {
            self.pos.clear();
            self.heading.clear();
            self.thrusters.clear();
        }
        if ui.button("Speed 0").clicked() {
            _ = self.sender.send(ServerRequest::SetSimSpeed(0));
        }
        if ui.button("Speed 1").clicked() {
            _ = self.sender.send(ServerRequest::SetSimSpeed(1));
        }
        if ui.button("Speed 5").clicked() {
            _ = self.sender.send(ServerRequest::SetSimSpeed(5));
        }
        if ui.button("Speed 20").clicked() {
            _ = self.sender.send(ServerRequest::SetSimSpeed(20));
        }
        if ui.button("Speed 50").clicked() {
            _ = self.sender.send(ServerRequest::SetSimSpeed(50));
        }
        if ui.button("Spawn Ship").clicked() {
            _ = self
                .sender
                .send(WorldDelta::Ping(randvec(100.0, 1000.0).into()).into());
        }

        ui.separator();

        for name in ["scenario_c", "scenario_d", "scenario_e"] {
            let path = format!("saves/{}.yaml", name);
            if ui.button(format!("Load {}", name)).clicked() {
                _ = self.sender.send(ServerRequest::LoadSave(path));
            }
        }

        ui.separator();

        if ui.button("Download World").clicked() {
            _ = self.sender.send(ServerRequest::BeginWorldDownload);
        }

        if ui.button("Request New Params").clicked() {
            _ = self.sender.send(ServerRequest::RequestParams);
        }
    }

    fn on_server_msg(&mut self, msg: ServerResponse) {
        let disc = msg.id();
        self.telemetry.insert(disc, msg.clone());
        self.logs.push_back(msg.clone());
        if self.logs.len() > 300 {
            self.logs.pop_front();
        }

        self.comparator.on_server_telemetry(&msg);

        match msg {
            ServerResponse::Driver { ticks, deltas, .. } => {
                self.server_ticks = ticks;

                for delta in deltas {
                    if let Err(e) = apply_delta(&mut self.world, delta) {
                        error!("Failed to apply delta from server: {e}");
                    }
                }

                while self.world.ticks < self.server_ticks {
                    update_world(&mut self.world);
                    self.comparator.update_local(&self.world);
                }
            }
            ServerResponse::Server(tlm) => {
                self.n_clients = tlm.n_clients;
            }
            ServerResponse::Thrusters {
                tick,
                grid,
                thrusters,
            } => {
                let x = tick as f64;
                for (id, throttle) in thrusters {
                    let y = throttle as f64;
                    let key = format!("gr-{}-thr-{}", grid, id);
                    self.thrusters.add(&key, x, y);
                }
            }
            ServerResponse::Grids(tlm) => {
                self.grid_info = tlm.grids.clone();
                for (id, gtlm) in tlm.grids {
                    let pos = gtlm.pos.tr();
                    self.pos
                        .add(&format!("pos_{}_{}", id, gtlm.name), pos.x, pos.y);
                    let x = tlm.tick as f64;
                    let y = wrap_0_2pi_f64(gtlm.pos.rotation).to_degrees();
                    self.heading
                        .add(&format!("heading_{}_{}", id, gtlm.name), x, y);
                    let y = gtlm.active_thrusters as f64;
                    self.thrusters.add(&format!("{}-tr#", id), x, y);
                }
            }
            ServerResponse::Parameters(params) => {
                warn!("Unused parameters: {params:?}");
            }
            ServerResponse::HaveNewSave => {
                _ = self.sender.send(ServerRequest::BeginWorldDownload);
            }
            ServerResponse::Ack => {
                // don't really care
            }
            ServerResponse::Entity(ident, id, data) => {
                info!("Got entity data: {} {} ({} bytes)", ident, id, data.len());
                if let Some(dl) = self.download.as_mut() {
                    dl.add_entity(ident, id, data);
                }
            }
            ServerResponse::TransactionStart(n, ticks) => {
                self.download = Some(WorldDownloader::new(n, ticks));
            }
            ServerResponse::TransactionComplete(_checksum) => {
                if let Some(dl) = std::mem::take(&mut self.download) {
                    self.world = dl.complete();
                    self.comparator.clear();
                    self.pos.clear();
                    self.heading.clear();
                    self.thrusters.clear();
                }
            }
            e => {
                info!("{e:?}");
            }
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for msg in self.receiver.try_iter().collect::<Vec<_>>() {
            self.on_server_msg(msg);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.request_repaint();
        // ui.set_zoom_factor(1.5);

        let mut dock_state = self.dock_state.clone();
        DockArea::new(&mut dock_state)
            .style(Style::from_egui(ui.style().as_ref()))
            .show_inside(ui, self);
        self.dock_state = dock_state;
    }
}

fn main() -> eframe::Result {
    _ = simple_logger::init_with_level(Level::Info);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_resizable(true),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Server Debugger",
        options,
        Box::new(|_| Ok(Box::<App>::default())),
    )
}
