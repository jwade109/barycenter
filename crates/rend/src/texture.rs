use crate::*;
use bary_core::prelude::Ent;
use glam::UVec2;
use image::GenericImageView;
use wgpu::*;

pub struct Texture {
    pub size: UVec2,
    pub bind_group: wgpu::BindGroup,
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn load_sprite(filename: &str, rd: &Renderer) -> Option<Self> {
        Texture::new_sprite(filename, &rd.device, &rd.queue, filename)
    }

    pub fn make_bind_group_layout(device: &Device, label: &str) -> BindGroupLayout {
        let mut builder: BindGroupLayoutBuilder<'_> = BindGroupLayoutBuilder::new(&device);
        builder.add_material();
        builder.build(label)
    }

    fn new_sprite(
        filename: &str,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
    ) -> Option<Self> {
        let bgl = Self::make_bind_group_layout(device, filename);

        let bytes = std::fs::read(filename).ok()?;
        let loaded_image = image::load_from_memory(&bytes).ok()?;
        let converted = loaded_image.to_rgba8();
        let size = loaded_image.dimensions();
        let texture_size = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        };

        // Create the texture
        let texture_descriptor = wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(label),
            view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
        };
        let texture = device.create_texture(&texture_descriptor);

        // Upload to it
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &converted,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * size.0),
                rows_per_image: Some(size.1),
            },
            texture_size,
        );

        // Get a view of the texture
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Make a sampler
        let sampler_descriptor = wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        };

        let sampler = device.create_sampler(&sampler_descriptor);

        let mut builder = BindGroupBuilder::new(device);
        builder.set_layout(&bgl);
        builder.add_material(&view, &sampler);
        let bind_group = builder.build(label);

        Some(Texture {
            size: UVec2::new(size.0, size.1),
            texture,
            bind_group,
            view,
            sampler,
        })
    }

    pub fn get_sample_range(&self) -> TextureSampleRange {
        TextureSampleRange {
            origin_x: 0,
            origin_y: 0,
            sample_width: self.size.x,
            sample_height: self.size.y,
            image_width: self.size.x,
            image_height: self.size.y,
        }
    }

    pub fn blank_texture(rd: &Renderer, width: u32, height: u32, label: &str) -> Self {
        let bgl = Self::make_bind_group_layout(&rd.device, label);

        let texture_descriptor = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: Some(label),
            view_formats: &[wgpu::TextureFormat::Bgra8UnormSrgb],
        };

        let texture = rd.device.create_texture(&texture_descriptor);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = rd.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let mut builder = BindGroupBuilder::new(&rd.device);
        builder.set_layout(&bgl);
        builder.add_material(&view, &sampler);
        let bind_group = builder.build(label);

        Self {
            size: UVec2::new(width, height),
            bind_group,
            texture,
            view,
            sampler,
        }
    }

    pub fn depth_texture(rd: &Renderer, label: &str) -> Self {
        let bgl = BindGroupLayoutBuilder::new(&rd.device).build(label);

        let size = wgpu::Extent3d {
            width: rd.config.width.max(1),
            height: rd.config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = rd.device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = rd.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        let mut builder = BindGroupBuilder::new(&rd.device);
        builder.set_layout(&bgl);
        // builder.add_material(&view, &sampler);
        let bind_group = builder.build(label);

        let size = (rd.config.width, rd.config.height);

        Self {
            size: UVec2::new(size.0, size.1),
            texture,
            view,
            sampler,
            bind_group,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureHandle {
    pub id: Ent,
    pub size: UVec2,
}
