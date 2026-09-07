use crate::Texture;

pub struct Renderer<'a> {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

impl<'a> Renderer<'a> {
    pub async fn new(window: &mut glfw::Window) -> Self {
        let instance_descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        };
        let instance = wgpu::Instance::new(instance_descriptor);
        let surface = instance.create_surface(window.render_context()).unwrap();

        let device_descriptor = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::POLYGON_MODE_LINE
                | wgpu::Features::POLYGON_MODE_POINT
                | wgpu::Features::BUFFER_BINDING_ARRAY,
            required_limits: wgpu::Limits {
                max_bind_groups: 8,
                ..Default::default()
            },
            memory_hints: wgpu::MemoryHints::Performance,
            label: Some("Device"),
        };

        let adapter_descriptor = wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        };
        let adapter = instance.request_adapter(&adapter_descriptor).await.unwrap();

        let (device, queue) = adapter
            .request_device(&device_descriptor, None)
            .await
            .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_capabilities.formats[0]);

        let size = window.get_framebuffer_size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0 as u32,
            height: size.1 as u32,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        Renderer {
            instance,
            surface,
            device,
            queue,
            config,
        }
    }

    pub fn make_command_encoder(&self) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }

    pub fn submit(&self, encoder: wgpu::CommandEncoder) {
        self.queue.submit(std::iter::once(encoder.finish()));
    }

    pub fn make_texture(&self, width: u32, height: u32, label: &str) -> Texture {
        Texture::blank_texture(self, width, height, label)
    }

    pub fn get_render_pass<'b>(
        &self,
        command_encoder: &'b mut wgpu::CommandEncoder,
        clear_color: Option<wgpu::Color>,
        view: &wgpu::TextureView,
        depth_texture: &Texture,
        clear_depth: bool,
    ) -> wgpu::RenderPass<'b> {
        let load = if clear_depth {
            wgpu::LoadOp::Clear(1.0)
        } else {
            wgpu::LoadOp::Load
        };

        let depth_stencil_attachment = None;

        let load = clear_color.map_or(wgpu::LoadOp::Load, |c| wgpu::LoadOp::Clear(c));

        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment,
            occlusion_query_set: None,
            timestamp_writes: None,
        };

        command_encoder.begin_render_pass(&render_pass_descriptor)
    }
}
