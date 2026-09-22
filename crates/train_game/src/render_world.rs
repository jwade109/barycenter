use crate::{
    event_bus::{EventBus, TrainEvent},
    terrain::ChunkIndex,
    world::World,
};
use bary_core::prelude::{Components, Ent, EntitySpawner};
use glam::UVec2;
use log::*;
use rend::*;
use std::{collections::BTreeMap, rc::Rc};

pub struct RenderWorld {
    pub fonts: Components<(FontInfo, Texture)>,
    pub meshes: Components<Mesh>,
    pub textures: Components<Texture>,
    pub spawner: EntitySpawner,
}

impl RenderWorld {
    pub fn new() -> Self {
        Self {
            fonts: Components::default(),
            meshes: Components::default(),
            textures: Components::default(),
            spawner: EntitySpawner::default(),
        }
    }

    pub fn spawn_mesh(&mut self, mesh: Mesh) -> Ent {
        let id = self.spawner.spawn();
        self.meshes.spawn(id, mesh);
        id
    }

    pub fn load_texture(
        &mut self,
        rd: &Renderer,
        path: &str,
        pixel_perfect: bool,
    ) -> TextureHandle {
        let mode = if pixel_perfect {
            wgpu::FilterMode::Nearest
        } else {
            wgpu::FilterMode::Linear
        };
        let sprite = Texture::load_sprite(path, rd, mode).unwrap();
        let size = sprite.size;
        let id = self.spawner.spawn();
        self.textures.spawn(id, sprite);

        TextureHandle { id, size }
    }

    pub fn load_font(&mut self, rd: &Renderer, name: &str) -> Ent {
        let data_path = format!("assets/font_textures/{name}/font_data.json");
        let texture_path = format!("assets/font_textures/{name}/font.png");
        let texture = Texture::load_sprite(&texture_path, rd, wgpu::FilterMode::Linear).unwrap();
        let font = FontInfo::from_file(&data_path).unwrap();
        let id = self.spawner.spawn();
        self.fonts.spawn(id, (font, texture));
        id
    }
}
