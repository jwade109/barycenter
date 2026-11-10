use bary_core::prelude::{ParamValue, randvec};
use bary_ipc::{ClientNode, ServerRequest, ServerResponse};
use bary_sim::WorldDelta;
use eframe::egui;
use egui_plot::*;
use log::{Level, info};
use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::*;
use std::time::{Duration, Instant};

mod backend_handle;

struct ParamEditorApp {
    start: Instant,
    sender: Sender<ServerRequest>,
    receiver: Receiver<ServerResponse>,
    text_fields: BTreeMap<String, ParameterField>,
    telemetry: BTreeMap<u8, ServerResponse>,
    logs: VecDeque<ServerResponse>,
    plots: BTreeMap<String, Vec<PlotPoint>>,
}

impl Default for ParamEditorApp {
    fn default() -> Self {
        let client = ClientNode::localhost(5000);

        let (request_sender, request_receiver) = channel();
        let (response_sender, response_receiver) = channel();

        std::thread::spawn(|| {
            background_thread(client, request_receiver, response_sender);
        });

        Self {
            start: Instant::now(),
            sender: request_sender,
            receiver: response_receiver,
            text_fields: BTreeMap::new(),
            telemetry: BTreeMap::new(),
            logs: VecDeque::new(),
            plots: BTreeMap::new(),
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
    request_receiver: Receiver<ServerRequest>,
    response_sender: Sender<ServerResponse>,
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

impl ParamEditorApp {
    fn add_plot_point(&mut self, key: &str, x: f64, y: f64) {
        let p = PlotPoint { x, y };
        if let Some(plot) = self.plots.get_mut(key) {
            plot.push(p);
            if plot.len() > 100 {
                plot.remove(0);
            }
        } else {
            self.plots.insert(key.to_string(), vec![p]);
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
                let x = (Instant::now() - self.start).as_secs_f64();
                let y = ticks as f64;
                self.add_plot_point("server_ticks", x, y);
            }
            ServerResponse::Info { clients, .. } => {
                let x = (Instant::now() - self.start).as_secs_f64();
                let y = clients as f64;
                self.add_plot_point("clients", x, y);
            }
            ServerResponse::Parameters(params) => {
                info!("Got new parameters: {params:?}");
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

fn draw_plot(ui: &mut egui::Ui, plots: &BTreeMap<String, Vec<PlotPoint>>) {
    let legend = Legend::default().title("Legendary");
    Plot::new("my_plot")
        .height(300.0)
        // .width(1200.0)
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

impl eframe::App for ParamEditorApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for msg in self.receiver.try_iter().collect::<Vec<_>>() {
            self.on_server_msg(msg);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.request_repaint();
        ui.set_zoom_factor(1.5);

        egui::CentralPanel::default().show(ui, |ui| {
            ui.separator();

            let mut response = None;

            render_params(ui, &mut self.text_fields, &mut response);

            ui.separator();

            draw_plot(ui, &self.plots);

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Clear Plots").clicked() {
                    self.plots.clear();
                }
                if ui.button("Speed 0").clicked() {
                    _ = self.sender.send(ServerRequest::SetSimSpeed(0));
                }
                if ui.button("Speed 1").clicked() {
                    _ = self.sender.send(ServerRequest::SetSimSpeed(1));
                }
                if ui.button("Speed 2").clicked() {
                    _ = self.sender.send(ServerRequest::SetSimSpeed(2));
                }
                if ui.button("Speed 4").clicked() {
                    _ = self.sender.send(ServerRequest::SetSimSpeed(4));
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
                if ui.button("Load World").clicked() {
                    _ = self
                        .sender
                        .send(ServerRequest::LoadSave("saves/scenario_c.yaml".into()));
                }
                if ui.button("Download World").clicked() {
                    _ = self.sender.send(ServerRequest::BeginAsyncWorldDownload);
                }
            });

            ui.separator();

            if let Some((key, val)) = response {
                let msg = ServerRequest::SetParameters([(key, val)].into());
                _ = self.sender.send(msg);
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

            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("telemetry")
                .show(ui, |ui| {
                    egui::Grid::new("telemetry").num_columns(2).show(ui, |ui| {
                        for (id, log) in &self.telemetry {
                            ui.label(format!("{id}"));
                            ui.label(format!("{log:?}"));
                            ui.end_row();
                        }
                    });
                });

            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("logs")
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for log in &self.logs {
                        ui.label(format!("{:?}", log));
                    }
                });
        });
    }
}

fn main() -> eframe::Result {
    _ = simple_logger::init_with_level(Level::Info);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_maximized(true)
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Server Debugger",
        options,
        Box::new(|_| Ok(Box::<ParamEditorApp>::default())),
    )
}
