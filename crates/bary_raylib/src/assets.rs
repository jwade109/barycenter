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

fn load_font(raylib: &mut Raylib, filename: &str) -> Option<Rc<Font>> {
    let font = raylib
        .handle
        .load_font_ex(&raylib.thread, filename, 32, None)
        .ok()?;
    Some(Rc::new(font))
}

pub struct Raylib {
    handle: RaylibHandle,
    thread: RaylibThread,
}

impl Raylib {
    pub fn new(handle: RaylibHandle, thread: RaylibThread) -> Self {
        Self { handle, thread }
    }

    pub fn draw(&mut self, func: impl FnMut(RaylibDrawHandle)) {
        self.handle.draw(&self.thread, func);
    }

    pub fn handle(&self) -> &RaylibHandle {
        &self.handle
    }
}

pub fn load_assets(assets: &mut Assets, raylib: &mut Raylib) {
    debug!("Loading assets");

    assets.lato_regular = load_font(raylib, "assets/fonts/Lato-Regular.ttf");
    assets.fira_code = load_font(raylib, "assets/fonts/FiraCode-Bold.ttf");
    assets.consolas = load_font(raylib, "assets/fonts/Consolas-Regular.ttf");

    assets.ship_names = load_names_from_file("assets/ship_names.txt").unwrap_or(vec![]);

    assets.keybinds = load_keybinds_from_file("assets/keybinds.yaml").unwrap();

    let parts = load_parts_from_dir("assets/parts/").unwrap();

    for (name, _part) in parts {
        let skin_path = format!("assets/parts/{}/skin.png", name);
        if let Ok(tex) = raylib.handle.load_texture(&raylib.thread, &skin_path) {
            assets.part_textures.insert(name, tex);
        }
    }

    assets.terrain_spritesheet = raylib
        .handle
        .load_texture(&raylib.thread, "assets/terrain/terrain_sprites.png")
        .ok();

    assets.pipe_tilemap = raylib
        .handle
        .load_texture(&raylib.thread, "assets/pipe.png")
        .ok();
}
