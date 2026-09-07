use std::collections::BTreeMap;

use crate::*;
use glam::DVec2;
use wgpu::*;

pub struct RectanglePipeline {
    pipeline: RenderPipeline,
    mesh: Mesh,
}

const RECT_DATA_F32_COUNT: usize = 12;

pub fn to_packed_array(cmd: &RectCommand, screen_size: DVec2) -> [f32; RECT_DATA_F32_COUNT] {
    [
        cmd.pos.x as f32,
        cmd.pos.y as f32,
        cmd.dims.x as f32,
        cmd.dims.y as f32,
        cmd.color.r as f32,
        cmd.color.g as f32,
        cmd.color.b as f32,
        cmd.color.a as f32,
        cmd.angle as f32,
        screen_size.x as f32,
        screen_size.y as f32,
        cmd.z as f32,
    ]
}

pub struct RectDataBuffer {
    buffer: BufferResource,
}

impl RectDataBuffer {
    pub fn new(rd: &Renderer, n_rects: usize) -> Self {
        let buffer =
            BufferResource::new_array(&rd.device, n_rects, RECT_DATA_F32_COUNT * 4, "rect_data");

        Self { buffer }
    }

    pub fn buffer(&self) -> &BufferResource {
        &self.buffer
    }

    pub fn write(&self, queue: &Queue, commands: &[RectCommand], screen: DVec2) {
        let data: Vec<_> = commands
            .iter()
            .flat_map(|c| {
                to_packed_array(c, screen)
                    .iter()
                    .flat_map(|e| e.to_le_bytes())
                    .collect::<Vec<u8>>()
            })
            .collect();

        self.buffer.upload(queue, 0, &data);
    }
}

impl RectanglePipeline {
    pub const RECTS_PER_PASS: usize = 1300;

    pub fn new(rd: &Renderer, shader_path: &str) -> (Self, BufferResource) {
        let height_data =
            BufferResource::new_array(&rd.device, Self::RECTS_PER_PASS, 4 * 4, "height_data");

        let mesh = make_quad(&rd.device);

        let layout = BufferResource::make_layout(&rd.device);

        let mut builder = PipelineBuilder::new(&rd.device);
        let shader = Shader::from_path(shader_path).unwrap();

        builder.add_bind_group_layout(&layout);
        builder.add_bind_group_layout(&layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Lava Lamp Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        (Self { pipeline, mesh }, height_data)
    }

    pub fn draw(&self, rp: &mut RenderPass, n: usize, buffers: &[&BufferResource]) {
        rp.set_pipeline(&self.pipeline);

        for (i, buffer) in buffers.into_iter().enumerate() {
            rp.set_bind_group(i as u32, buffer.bind_group(), &[]);
        }

        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..n as u32);
    }
}
