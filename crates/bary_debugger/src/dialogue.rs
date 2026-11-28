use bary_core::prelude::BaryResult;
use bary_ipc::ServerRequest;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::mpsc::Sender,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dialogue {
    speech: BTreeMap<String, String>,
}

const PLACEHOLDER: &str = "<<< This is placeholder text! >>>";

impl Dialogue {
    pub fn new() -> Self {
        Self {
            speech: BTreeMap::new(),
        }
    }

    pub fn from_path(path: impl AsRef<Path>) -> BaryResult<Self> {
        let contents = std::fs::read_to_string(path)?;
        let speech = serde_yaml::from_str(&contents)?;
        Ok(Self { speech })
    }

    pub fn get(&self, id: &str) -> &str {
        self.speech
            .get(id)
            .map(|s| s.as_str())
            .unwrap_or(PLACEHOLDER)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.speech.iter()
    }
}

pub struct DialogueSender {
    path: String,
    dialogue: Option<Dialogue>,
}

impl DialogueSender {
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let dialogue = Dialogue::from_path(&path).ok();

        Self {
            path: path,
            dialogue,
        }
    }

    fn reload(&mut self) {
        self.dialogue = Dialogue::from_path(&self.path).ok();
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, sender: &mut Sender<ServerRequest>) {
        ui.text_edit_singleline(&mut self.path);

        if ui.button("Reload").clicked() {
            self.reload();
        }

        let Some(d) = &self.dialogue else {
            ui.label(format!("Failed to load dialogue at {}", self.path));
            return;
        };

        for (id, message) in d.iter() {
            let resp = ui.button(id);
            if resp.clicked() {
                _ = sender.send(ServerRequest::Rebroadcast(message.clone()));
            }
            if resp.hovered() {
                ui.label(message);
            }
        }
    }
}
