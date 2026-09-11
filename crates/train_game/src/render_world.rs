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
    pub memory: Components<BufferResource>,

    pub rect_data: RectDataBuffer,
    pub height_data_chunks: BufferResource,

    spawner: EntitySpawner,
}

impl RenderWorld {
    pub fn new(rect: RectDataBuffer, height: BufferResource) -> Self {
        Self {
            fonts: Components::default(),
            meshes: Components::default(),
            textures: Components::default(),
            memory: Components::default(),
            rect_data: rect,
            height_data_chunks: height,
            spawner: EntitySpawner::default(),
        }
    }

    pub fn spawn_mesh(&mut self, mesh: Mesh) -> Ent {
        let id = self.spawner.spawn();
        self.meshes.spawn(id, mesh);
        id
    }

    pub fn load_texture(&mut self, rd: &Renderer, path: &str) -> TextureHandle {
        let sprite = Texture::load_sprite(path, rd).unwrap();
        let size = sprite.size;
        let id = self.spawner.spawn();
        self.textures.spawn(id, sprite);

        TextureHandle { id, size }
    }

    pub fn load_font(&mut self, rd: &Renderer, name: &str) -> Ent {
        let data_path = format!("assets/font_textures/{name}/font_data.json");
        let texture_path = format!("assets/font_textures/{name}/font.png");
        let texture = Texture::load_sprite(&texture_path, rd).unwrap();
        let font = FontInfo::from_file(&data_path).unwrap();
        let id = self.spawner.spawn();
        self.fonts.spawn(id, (font, texture));
        id
    }

    pub fn create_memory_arena(
        &mut self,
        rd: &Renderer,
        name: impl Into<String>,
        size: usize,
    ) -> Ent {
        let name = name.into();
        let resource = BufferResource::new(&rd.device, size, &name);
        let id = self.spawner.spawn();
        self.memory.spawn(id, resource);
        id
    }

    pub fn handle_events(
        &mut self,
        rd: &Renderer,
        events: &EventBus<TrainEvent>,
        world: &mut World,
    ) {
        for event in events.iter() {
            if let TrainEvent::ChunkUpdate(id) = event {
                let Ok(chunk) = world.chunks.try_get_mut(*id) else {
                    continue;
                };

                let id = world.spawner.spawn();

                warn!("Spawning texture for chunk {:?}", chunk.index());

                let texture = rd.make_texture(500, 500, "");
                let handle = TextureHandle {
                    id,
                    size: texture.size,
                };
                self.textures.spawn(id, texture);
                chunk.texture = Some(handle);
            }
        }
    }
}
