use bary_core::prelude::ParamValue;
use bary_ipc::{ServerRequest, ClientNode, ServerMessage};
use eframe::egui;
use log::{Level, info};
use std::collections::BTreeMap;
use std::sync::mpsc::*;
use std::time::Duration;

mod backend_handle;

struct ParamEditorApp {
    sender: Sender<ServerRequest>,
    receiver: Receiver<ServerMessage>,
    text_fields: BTreeMap<String, ParameterField>,
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
            sender: request_sender,
            receiver: response_receiver,
            text_fields: BTreeMap::new(),
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
    response_sender: Sender<ServerMessage>,
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
    fn on_server_msg(&mut self, msg: ServerMessage) {
        match msg {
            ServerMessage::Driver { .. } => {
                // explicitly ignoring these messages
            }
            ServerMessage::Parameters(params) => {
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

impl eframe::App for ParamEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for msg in self.receiver.try_iter().collect::<Vec<_>>() {
            self.on_server_msg(msg);
        }

        ctx.request_repaint();
        ctx.set_zoom_factor(1.5);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.separator();

            let mut response = None;

            render_params(ui, &mut self.text_fields, &mut response);

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
