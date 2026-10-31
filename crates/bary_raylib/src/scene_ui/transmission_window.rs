use crate::*;
use bary_core::prelude::{Components, Ent};
use bary_sim::*;

pub fn make_transmission_ui(ui: &mut UiBuilder, trs: &Components<RadioTransmission>) {
    for (id, tr) in trs.iter() {
        make_transmission_window(ui, *id, tr);
    }
}

fn make_transmission_window(ui: &mut UiBuilder, id: Ent, tr: &RadioTransmission) {
    ui.window(id.0, |ui| {
        ui.text(format!("Transmission {id}"));
        ui.text(format!("{:#?}", tr));
        ui.button(UiMessage::AckTransmission(id), "Acknowledge");
    })
}
