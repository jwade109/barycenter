use bary_core::prelude::*;
use serde::{Deserialize, Serialize};

pub const SPEED_OF_LIGHT: f64 = 5000.0; // m/s

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum TransmissionContents {
    Nothing,
    Message(String),
    Response,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RadioTransmission {
    origin: DVec2,
    radius: f64,
    emission_power: f64,
    source: Option<Ent>,
    contents: TransmissionContents,
}

impl RadioTransmission {
    pub fn new(origin: DVec2) -> Self {
        Self {
            origin,
            radius: 0.0,
            emission_power: 1E6,
            source: None,
            contents: TransmissionContents::Nothing,
        }
    }

    pub fn with_range(mut self, range: f64) -> Self {
        self.emission_power = (1.0 + range).powi(2);
        self
    }

    pub fn with_source(mut self, source: impl Into<Option<Ent>>) -> Self {
        self.source = source.into();
        self
    }

    pub fn with_message(mut self, contents: impl Into<String>) -> Self {
        self.contents = TransmissionContents::Message(contents.into());
        self
    }

    pub fn with_contents(mut self, contents: TransmissionContents) -> Self {
        self.contents = contents;
        self
    }

    pub fn origin(&self) -> DVec2 {
        self.origin
    }

    pub fn radius(&self) -> f64 {
        self.radius
    }

    pub fn emission_power(&self) -> f64 {
        self.emission_power
    }

    pub fn received_power(&self) -> f64 {
        self.emission_power / (1.0 + self.radius).powi(2)
    }

    pub fn source(&self) -> Option<Ent> {
        self.source
    }

    pub fn contents(&self) -> &TransmissionContents {
        &self.contents
    }

    pub fn is_response(&self) -> bool {
        matches!(self.contents, TransmissionContents::Response)
    }

    pub fn update(&mut self, dt: f64) -> Option<(f64, f64)> {
        let old_radius = self.radius;
        self.radius += SPEED_OF_LIGHT * dt;
        (self.received_power() > 1.0).then_some((old_radius, self.radius))
    }
}

/// A construct for "sending" events or other
/// small things outside of the current context.
#[derive(Debug, Clone)]
pub struct Sender<T> {
    messages: Vec<T>,
}

impl<T> Sender<T> {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn send(&mut self, elem: T) {
        self.messages.push(elem);
    }

    pub fn take(self) -> Vec<T> {
        self.messages
    }
}
