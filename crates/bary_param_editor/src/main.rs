use bary_core::prelude::{Ent, Isometry2d, ParamValue, randvec, wrap_0_2pi_f64, wrap_pi_npi_f64};
use bary_ipc::{ClientNode, ServerRequest, ServerResponse};
use bary_sim::{Sender, WorldDelta};
use eframe::egui;
use egui::{Id, Ui, WidgetText};
use egui_dock::{DockArea, DockState, NodeIndex, Style, TabViewer};
use egui_plot::*;
use log::{Level, info};
use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc;
use std::time::Duration;

mod backend_handle;
mod grid_commander;
mod plotter;

use crate::grid_commander::GridCommander;
use crate::plotter::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Tab {
    PositionPlot,
    HeadingPlot,
    ThrustersPlot,
    Buttons,
    GridCommander,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tab::HeadingPlot => "Heading",
            Tab::PositionPlot => "Position",
            Tab::ThrustersPlot => "Thrusters",
            Tab::Buttons => "Buttons",
            Tab::GridCommander => "Grid Commander",
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
                self.heading.draw(ui, "heading", None);
            }
            Tab::PositionPlot => {
                self.pos.draw(ui, "pos", 1.0);
            }
            Tab::ThrustersPlot => {
                self.thrusters.draw(ui, "thrusters", None);
            }
            Tab::Buttons => {
                self.button_panel(ui);
            }
            Tab::GridCommander => {
                self.commander.draw(ui, &self.grid_info, &mut deltas);
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
    text_fields: BTreeMap<String, ParameterField>,
    telemetry: BTreeMap<u8, ServerResponse>,
    logs: VecDeque<ServerResponse>,

    grid_info: BTreeMap<Ent, (String, String, Isometry2d)>,

    server_ticks: u64,
    n_clients: usize,

    pos: Plotter,
    heading: Plotter,
    thrusters: Plotter,
    commander: GridCommander,

    dock_state: DockState<Tab>,
}

impl Default for App {
    fn default() -> Self {
        let client = ClientNode::localhost(5000);

        let (request_sender, request_receiver) = mpsc::channel();
        let (response_sender, response_receiver) = mpsc::channel();

        std::thread::spawn(|| {
            background_thread(client, request_receiver, response_sender);
        });

        let tabs = vec![Tab::PositionPlot, Tab::HeadingPlot, Tab::ThrustersPlot];
        let mut dock_state = DockState::new(tabs);

        // Currently, the `DockState` only has one `Surface`: the main one.
        // Let's get mutable access to add more nodes in it.
        let surface = dock_state.main_surface_mut();

        // Insert "tab2" to the left of "tab1", where the width of "tab2"
        // is 20% of root node's width.
        let [_old_node, _new_node] = surface.split_left(
            NodeIndex::root(),
            0.20,
            vec![Tab::Buttons, Tab::GridCommander],
        );

        Self {
            sender: request_sender,
            receiver: response_receiver,
            text_fields: BTreeMap::new(),
            telemetry: BTreeMap::new(),
            logs: VecDeque::new(),
            server_ticks: 0,
            n_clients: 0,
            pos: Plotter::new(),
            heading: Plotter::new(),
            thrusters: Plotter::new(),
            commander: GridCommander::new(),
            dock_state,
            grid_info: BTreeMap::new(),
        }
    }
}

fn param_field_from_text(s: &str) -> ParamValue {
    if let Ok(x) = s.parse::<i64>() {
        ParamValue::Integer(x)
    } else if let Ok(x) = s.parse::<f64>() {
        ParamValue::Float(x)
    } else {
        ParamValue::String(s.to_string())
    }
}

struct ParameterField {
    field: String,
    value: ParamValue,
    is_dirty: bool,
}

fn background_thread(
    mut client: ClientNode,
    request_receiver: mpsc::Receiver<ServerRequest>,
    response_sender: mpsc::Sender<ServerResponse>,
) {
    client.send(ServerRequest::Ping);
    client.send(ServerRequest::RequestParams);

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
        if ui.button("Say Hello").clicked() {
            _ = self
                .sender
                .send(ServerRequest::ClientSays("Hello there!".into()));
        }

        for name in ["scenario_c", "scenario_d", "scenario_e"] {
            let path = format!("saves/{}.yaml", name);
            if ui.button(format!("Load {}", name)).clicked() {
                _ = self.sender.send(ServerRequest::LoadSave(path));
            }
        }

        if ui.button("Download World").clicked() {
            _ = self.sender.send(ServerRequest::BeginAsyncWorldDownload);
        }

        if ui.button("Request New Params").clicked() {
            _ = self.sender.send(ServerRequest::RequestParams);
        }

        if ui.button("Push Params").clicked() {
            let mut params = BTreeMap::new();
            for (key, field) in &self.text_fields {
                params.insert(key.clone(), field.value.clone());
            }
            _ = self.sender.send(ServerRequest::SetParameters(params));
        }
    }

    fn on_server_msg(&mut self, msg: ServerResponse) {
        let disc = msg.id();
        self.telemetry.insert(disc, msg.clone());
        self.logs.push_back(msg.clone());
        if self.logs.len() > 300 {
            self.logs.pop_front();
        }

        match msg {
            ServerResponse::Driver { ticks, .. } => {
                self.server_ticks = ticks;
            }
            ServerResponse::Info { clients, .. } => {
                // let x = (Instant::now() - self.start).as_secs_f64();
                // let y = clients as f64;
                // self.metadata.add("clients", x, y);
                self.n_clients = clients;
            }
            ServerResponse::Thrusters {
                tick,
                grid,
                thrusters,
            } => {
                let x = tick as f64;
                let y = thrusters.iter().filter(|(_, t)| **t > 0.0).count() as f64;
                self.thrusters.add(&format!("{}-tr#", grid), x, y);
                for (id, throttle) in thrusters {
                    let y = throttle as f64;
                    let key = format!("gr-{}-thr-{}", grid, id);
                    self.thrusters.add(&key, x, y);
                }
            }
            ServerResponse::Grids { tick, grids } => {
                self.grid_info = grids.clone();
                for (id, (name, _model, iso)) in grids {
                    let pos = iso.tr();
                    self.pos.add(&format!("pos_{}_{}", id, name), pos.x, pos.y);
                    let x = tick as f64;
                    let y = wrap_0_2pi_f64(iso.rotation).to_degrees();
                    self.heading.add(&format!("heading_{}_{}", id, name), x, y);
                }
            }
            ServerResponse::Parameters(params) => {
                for (key, value) in params {
                    self.text_fields
                        .entry(key)
                        .and_modify(|f| {
                            f.value = value.clone();
                            f.field = value.to_string();
                        })
                        .or_insert(ParameterField {
                            field: value.to_string(),
                            value: value,
                            is_dirty: false,
                        });
                }
            }
            ServerResponse::HaveNewSave => {
                self.pos.clear();
                self.heading.clear();
                self.thrusters.clear();
            }
            ServerResponse::Ack => {
                // don't really care
            }
            e => {
                info!("{e:?}");
            }
        }
    }
}

fn render_params(
    ui: &mut egui::Ui,
    text_fields: &mut BTreeMap<String, ParameterField>,
    response: &mut Option<(String, ParamValue)>,
) {
    egui::Grid::new("params")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for (key, field) in text_fields {
                ui.label(key);
                if ui.text_edit_singleline(&mut field.field).changed() {
                    field.is_dirty = true;
                }
                field.value = param_field_from_text(&field.field);
                ui.label(format!("{:?}", field.value));
                let button = egui::Button::new("Update");
                if ui.add_enabled(field.is_dirty, button).clicked() {
                    field.is_dirty = false;
                    *response = Some((key.clone(), field.value.clone()));
                }
                ui.end_row();
            }
        });
}

fn draw_plot(ui: &mut egui::Ui, plots: &BTreeMap<String, Vec<PlotPoint>>, id: &str) {
    let legend = Legend::default();
    Plot::new(id)
        .legend(legend)
        .show_crosshair(false)
        .show_grid([false, false])
        .show(ui, |plot_ui| {
            for (name, data) in plots {
                let points = Points::new(name, data.as_slice()).radius(3.0);
                plot_ui.points(points);
                let series = Line::new(name, data.as_slice());
                plot_ui.line(series);
            }
        });
}

impl eframe::App for App {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for msg in self.receiver.try_iter().collect::<Vec<_>>() {
            self.on_server_msg(msg);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.request_repaint();
        ui.set_zoom_factor(1.5);

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
