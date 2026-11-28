use crate::utils::{GlobalKeybinds, load_keybinds_from_file};
use bary_parts::load_parts_from_dir;
use bary_sim::load_names_from_file;
use log::debug;
use raylib::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

pub type MaybeTexture = Option<Texture2D>;

pub type MaybeFont = Option<Font>;

pub struct Assets {
    lato_regular: Option<Rc<Font>>,
    fira_code: Option<Rc<Font>>,
    consolas: Option<Rc<Font>>,
    pub thruster_shader: Shader,
    pub part_textures: BTreeMap<String, Texture2D>,
    pub __animation: MaybeTexture,
    pub terrain_spritesheet: MaybeTexture,
    pub pipe_tilemap: MaybeTexture,
    pub blob: MaybeTexture,
    pub ship_names: Vec<String>,
    pub keybinds: GlobalKeybinds,
}

impl Assets {
    pub fn load(raylib: &mut Raylib) -> Self {
        debug!("Loading assets");

        let lato_regular = load_font(raylib, "assets/fonts/Lato-Regular.ttf");
        let fira_code = load_font(raylib, "assets/fonts/FiraCode-Bold.ttf");
        let consolas = load_font(raylib, "assets/fonts/Consolas-Regular.ttf");

        let ship_names = load_names_from_file("assets/ship_names.txt").unwrap_or(vec![]);

        let keybinds = load_keybinds_from_file("assets/keybinds.yaml").unwrap();

        let parts = load_parts_from_dir("assets/parts/").unwrap();

        let mut part_textures = BTreeMap::new();

        for (name, _part) in parts {
            let skin_path = format!("assets/parts/{}/skin.png", name);
            if let Ok(tex) = raylib.handle.load_texture(&raylib.thread, &skin_path) {
                part_textures.insert(name, tex);
            }
        }

        let thruster_shader =
            raylib
                .handle
                .load_shader(&raylib.thread, None, Some("assets/shaders/thruster.fs"));

        let terrain_spritesheet = raylib
            .handle
            .load_texture(&raylib.thread, "assets/terrain/terrain_sprites.png")
            .ok();

        let pipe_tilemap = raylib
            .handle
            .load_texture(&raylib.thread, "assets/pipe.png")
            .ok();

        let blob = raylib
            .handle
            .load_texture(&raylib.thread, "assets/lado.png")
            .ok();

        Self {
            lato_regular,
            fira_code,
            consolas,
            thruster_shader,
            part_textures,
            __animation: None,
            terrain_spritesheet,
            pipe_tilemap,
            blob,
            ship_names,
            keybinds,
        }
    }

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

    pub fn init() -> Self {
        let (mut handle, thread) = raylib::init()
            .size(1080, 700)
            .title("Barycenter")
            .log_level(TraceLogLevel::LOG_WARNING)
            .msaa_4x()
            .resizable()
            .build();

        handle.set_target_fps(1000);
        handle.maximize_window();
        handle.set_exit_key(None);
        handle.hide_cursor();

        Self { handle, thread }
    }

    pub fn draw(&mut self, func: impl FnMut(RaylibDrawHandle)) {
        self.handle.draw(&self.thread, func);
    }

    pub fn handle(&self) -> &RaylibHandle {
        &self.handle
    }
}
