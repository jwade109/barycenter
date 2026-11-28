use crate::{Inventory, MachineStatus, Recipe, RecipeListing};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Machine {
    pub enabled: bool,
    pub steps: u32,
    pub required_steps: u32,
    pub recipe: RecipeListing,
    pub products_finished: u64,
    pub status: MachineStatus,
    pub input_slots: Vec<usize>,
    pub output_slots: Vec<usize>,
}

impl Machine {
    pub fn is_running(&self) -> bool {
        self.status == MachineStatus::Running
    }

    pub fn progress(&self) -> f32 {
        self.steps as f32 / self.required_steps as f32
    }

    pub fn set_recipe(&mut self, recipe: RecipeListing) {
        self.recipe = recipe;
        self.steps = 0;
    }

    pub fn recipe(&self) -> Option<Recipe> {
        if self.recipe == RecipeListing::DoNothing {
            None
        } else {
            Some(self.recipe.to_recipe())
        }
    }

    pub fn take_inputs_if_possible(&self, inv: &mut Inventory) -> bool {
        let recipe = match self.recipe() {
            Some(r) => r,
            _ => return false,
        };

        for (i, (item, count)) in recipe.inputs().enumerate() {
            let Some(slot_index) = self.input_slots.get(i) else {
                return false;
            };
            let Some(slot) = inv.get_slot(*slot_index) else {
                return false;
            };
            if !slot.can_take(item, count) {
                return false;
            }
        }

        for (i, (item, count)) in recipe.inputs().enumerate() {
            let Some(slot_index) = self.input_slots.get(i) else {
                return false;
            };
            let Some(slot) = inv.get_slot_mut(*slot_index) else {
                return false;
            };
            if !slot.take(item, count) {
                return false;
            }
        }

        true
    }

    pub fn store_outputs_if_possible(&self, inv: &mut Inventory) -> bool {
        let recipe = match self.recipe() {
            Some(r) => r,
            _ => return false,
        };

        for (i, (item, count)) in recipe.outputs().enumerate() {
            let Some(slot_index) = self.output_slots.get(i) else {
                return false;
            };
            let Some(slot) = inv.get_slot(*slot_index) else {
                return false;
            };
            if slot.can_store(item, count).is_err() {
                return false;
            }
        }

        for (i, (item, count)) in recipe.outputs().enumerate() {
            let Some(slot_index) = self.output_slots.get(i) else {
                return false;
            };
            let Some(slot) = inv.get_slot_mut(*slot_index) else {
                return false;
            };
            if slot.store(item, count).is_err() {
                return false;
            }
        }

        true
    }
}
