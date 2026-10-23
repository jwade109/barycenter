use crate::utils::{GlobalKeybinds, load_keybinds_from_file};
use bary_parts::load_parts_from_dir;
use bary_sim::load_names_from_file;
use log::debug;
use raylib::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

pub type MaybeTexture = Option<Texture2D>;

pub type MaybeFont = Option<Font>;

#[derive(Default)]
pub struct Assets {
    lato_regular: MaybeFont,
    fira_code: MaybeFont,
    consolas: Option<Rc<Font>>,
    pub part_textures: BTreeMap<String, Texture2D>,
    pub animation: MaybeTexture,
    pub terrain_spritesheet: MaybeTexture,
    pub pipe_tilemap: MaybeTexture,
    pub ship_names: Vec<String>,
    pub keybinds: GlobalKeybinds,
}

impl Assets {
    pub fn ui_font(&self) -> Rc<Font> {
        self.consolas.clone().unwrap()
    }
}

pub fn load_assets(
    assets: &mut Assets,
    rl: &mut raylib::RaylibHandle,
    thread: &raylib::RaylibThread,
) {
    debug!("Loading assets");

    assets.lato_regular = rl
        .load_font_ex(thread, "assets/fonts/Lato-Regular.ttf", 32, None)
        .ok();

    assets.fira_code = rl
        .load_font_ex(thread, "assets/fonts/FiraCode-Bold.ttf", 32, None)
        .ok();

    let consolas = rl
        .load_font_ex(thread, "assets/fonts/Consolas-Regular.ttf", 32, None)
        .ok();

    assets.consolas = consolas.map(|c| Rc::new(c));

    assets.ship_names = load_names_from_file("assets/ship_names.txt").unwrap_or(vec![]);

    assets.keybinds = load_keybinds_from_file("assets/keybinds.yaml").unwrap();

    let parts = load_parts_from_dir("assets/parts/").unwrap();

    for (name, _part) in parts {
        let skin_path = format!("assets/parts/{}/skin.png", name);
        if let Ok(tex) = rl.load_texture(thread, &skin_path) {
            assets.part_textures.insert(name, tex);
        }
    }

    assets.terrain_spritesheet = rl
        .load_texture(&thread, "assets/terrain/terrain_sprites.png")
        .ok();

    assets.pipe_tilemap = rl.load_texture(thread, "assets/pipe.png").ok();
}
