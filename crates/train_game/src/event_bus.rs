use bary_core::prelude::Ent;
use log::info;

use crate::{sounds::SoundKind, terrain::ChunkIndex};

pub struct FontSelection {
    font_id: Option<Ent>,
}

impl FontSelection {
    pub fn new() -> Self {
        Self { font_id: None }
    }

    pub fn clicked(&mut self, font_id: Ent) {
        self.font_id = Some(font_id);
    }

    pub fn new_font_id(&self) -> Option<Ent> {
        self.font_id
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TrainEvent {
    ChunkUpdate(Ent),
    Sound(SoundKind),
    CarReparent(Ent),
    NewConsist(Ent),
    RegenerateTrees(ChunkIndex),
    RedrawTile(ChunkIndex),
    KillSound(Ent),
    ToggleDebug,
    ToggleDetail,
    Other,
}

pub struct EventBus<T> {
    events: Vec<T>,
}

impl<T: std::fmt::Debug> EventBus<T> {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn enqueue(&mut self, event: T) {
        info!("E: {event:?}");
        self.events.push(event);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.events.iter()
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}
