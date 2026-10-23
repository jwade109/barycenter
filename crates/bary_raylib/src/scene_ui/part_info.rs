use crate::*;
use bary_core::prelude::Ent;
use bary_factory::InvSlot;
use bary_input::InputState;
use bary_sim::*;
use bary_ui::*;
use log::warn;
use rdev::EventType;

pub fn part_info_modal(
    part_id: Ent,
    ui: &UiBuilder,
    world: &World,
    assets: &Assets,
    cmd: &str,
) -> Option<Node<UiMessage>> {
    let part = world.parts.try_get(part_id).ok()?;
    let proto = world.prototypes.try_get(part.prototype).ok()?;

    let mut children = Vec::new();

    children.push(ui.text(proto.name.clone()));

    if let Some(texture) = assets.part_textures.get(&proto.name) {
        children.push(ui.sprite(proto.name.clone(), texture));
    }

    if let Ok(cpu) = world.computers.try_get(part_id) {
        children.push(ui.button(
            UiMessage::SetComputerOnOff(part_id, !cpu.on),
            if cpu.on { "Turn Off" } else { "Turn On" },
        ));
        children.push(ui.button(UiMessage::SetComputerDrift(part_id), "Set to Drift"));
        let info = computer_info_str(cpu);
        children.push(ui.text(info));
    }
    if let Ok(thruster) = world.thrusters.try_get(part_id) {
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
    if let Ok(light) = world.lights.try_get(part_id) {
        let s = format!("{:#?}", light);
        children.push(ui.text(s));
    }
    if let Ok(mac) = world.machines.try_get(part_id) {
        children.push(ui.progress_bar(mac.progress()));
        children.push(ui.button(
            UiMessage::SetMachineState(part_id, !mac.enabled),
            if mac.enabled { "Turn Off" } else { "Turn On" },
        ));
    }
    if let Ok(inv) = world.inventories.try_get(part_id) {
        for (idx, slot) in inv.slots().enumerate() {
            let line = slot_info_str(idx, slot);
            children.push(ui.text(line));
            children.push(ui.progress_bar(slot.fill_percentage()));
        }
    }

    children.push(ui.text(cmd));

    Some(Node::column(600.0, children))
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

pub fn part_focus_terminal_input(state: &mut PartFocusState, input: &InputState) {
    if let Some(cmd) = single_line_string_edit(&mut state.command_prompt, input) {
        warn!("WOW: {}", cmd);
    }
}

pub fn single_line_string_edit(s: &mut String, input: &InputState) -> Option<String> {
    for event in input.events() {
        if let EventType::KeyPress(k) = event.event_type {
            match k {
                rdev::Key::Backspace => {
                    s.pop();
                }
                rdev::Key::Return => return Some(std::mem::take(s)),
                rdev::Key::BackQuote => (),
                rdev::Key::Escape => (),
                rdev::Key::Tab => (),
                rdev::Key::UpArrow => (),
                rdev::Key::DownArrow => (),
                _ => {
                    if let Some(n) = &event.name {
                        if n.is_ascii() {
                            s.extend(n.chars());
                        }
                    }
                }
            }
        }
    }
    None
}
