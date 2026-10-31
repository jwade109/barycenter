use crate::*;
use bary_core::prelude::{Components, Ent};
use bary_sim::*;

pub fn make_transmission_ui(ui: &mut UiBuilder, trs: &Components<ReceivedTransmission>) {
    let mut i = 0;
    for (id, tr) in trs.iter() {
        i += 1;
        make_transmission_window(ui, *id, tr);
        if i == 3 {
            break;
        }
    }
}

fn make_transmission_window(ui: &mut UiBuilder, id: Ent, tr: &ReceivedTransmission) {
    ui.window(id.0, |ui| {
        ui.text(format!("Transmission {id}"));
        ui.text(format!("{:#?}", tr));
        ui.button(UiMessage::AckTransmission(id), "Acknowledge");
        ui.button(UiMessage::ClearTransmissions, "Clear All");
    })
}
