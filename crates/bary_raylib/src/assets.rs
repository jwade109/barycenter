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
    lato_regular: Option<Rc<Font>>,
    fira_code: Option<Rc<Font>>,
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

fn load_font(rl: &mut RaylibHandle, thread: &RaylibThread, filename: &str) -> Option<Rc<Font>> {
    let font = rl.load_font_ex(thread, filename, 32, None).ok()?;
    Some(Rc::new(font))
}

pub fn load_assets(
    assets: &mut Assets,
    rl: &mut raylib::RaylibHandle,
    thread: &raylib::RaylibThread,
) {
    debug!("Loading assets");

    assets.lato_regular = load_font(rl, thread, "assets/fonts/Lato-Regular.ttf");
    assets.fira_code = load_font(rl, thread, "assets/fonts/FiraCode-Bold.ttf");
    assets.consolas = load_font(rl, thread, "assets/fonts/Consolas-Regular.ttf");

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
