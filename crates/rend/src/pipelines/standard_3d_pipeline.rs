use crate::Texture;
use crate::*;
use bary_core::prelude::rand;
use wgpu::util::DeviceExt;
use wgpu::*;

pub struct Standard3DPipeline {
    pipeline: RenderPipeline,
    transforms: BufferResource,
}

impl Standard3DPipeline {
    pub const MAX_MESHES_PER_PASS: usize = 100;

    pub fn new(rd: &Renderer) -> Self {
        let transforms = BufferResource::new_array(
            &rd.device,
            Self::MAX_MESHES_PER_PASS,
            Transform32::LAYOUT_SIZE,
            "mesh.transforms",
        );

        let tf_layout = BufferResource::make_layout(&rd.device);

        let mut builder = PipelineBuilder::new(&rd.device);
        let shader = Shader::from_path("crates/rend/shaders/mesh.wgsl").unwrap();

        builder.add_bind_group_layout(&tf_layout);

        let pipeline = builder.build_pipeline::<FullVertex>(
            "Standard 3D Pipeline",
            &shader,
            rd.config.format,
            true,
            true,
        );

        Self {
            pipeline,
            transforms,
        }
    }

    pub fn draw(&self, queue: &Queue, rp: &mut RenderPass, tf: &Transform32, mesh: &Mesh) {
        rp.set_pipeline(&self.pipeline);
        mesh.set_as_active(rp);
        self.transforms.upload(queue, 0, &tf.into_gpu());
        rp.set_bind_group(0, self.transforms.bind_group(), &[]);
        rp.draw_indexed(0..mesh.index_count(), 0, 0..1);
    }
}
