use crate::Texture;
use crate::*;
use glam::Vec2;
use wgpu::*;

pub struct BlurPipeline {
    pipeline: RenderPipeline,
    params: BufferResource,
    mesh: Mesh,
}

pub struct BlurParams {
    pub resolution: Vec2,
    pub kernel_size: f32,
    pub is_vertical: bool,
    pub is_nullopt: bool,
}

impl BlurParams {
    pub fn to_gpu(&self) -> Vec<u8> {
        [
            self.resolution.x.to_le_bytes(),
            self.resolution.y.to_le_bytes(),
            self.kernel_size.to_le_bytes(),
            (self.is_vertical as i32).to_le_bytes(),
            (self.is_nullopt as i32).to_le_bytes(),
        ]
        .concat()
    }
}

impl BlurPipeline {
    pub fn new(device: &Device, config: &SurfaceConfiguration) -> Self {
        let mesh = make_quad(device);
        let bgl = Texture::make_bind_group_layout(device, "BlurPipeline Bind Group Layout");
        let shader = Shader::from_path("crates/rend/shaders/blur_shader.wgsl").unwrap();

        let params = BufferResource::new(device, 32, "Blur Pipeline Params");

        let layout = BufferResource::make_layout(device);

        let mut builder = PipelineBuilder::new(&device);
        builder.add_bind_group_layout(&bgl);
        builder.add_bind_group_layout(&layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Gaussian Blur Pipeline",
            &shader,
            config.format,
            true,
            true,
        );

        Self {
            pipeline,
            params,
            mesh,
        }
    }

    pub fn blur_pass(
        &self,
        rp: &mut RenderPass,
        queue: &Queue,
        params: BlurParams,
        material: &BindGroup,
    ) {
        self.params.write(queue, &params.to_gpu());
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, material, &[]);
        rp.set_bind_group(1, self.params.bind_group(), &[]);
        self.mesh.set_as_active(rp);
        rp.draw_indexed(0..self.mesh.index_count(), 0, 0..1);
    }
}
