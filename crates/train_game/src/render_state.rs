use crate::{render_world::RenderWorld, terrain::*, viewport::Viewport, *};
use glam::DVec2;
use wgpu::SurfaceTexture;

struct Pipelines {
    lava_lamp_pipeline: LavaLampPipeline,
    blur_pipeline: BlurPipeline,
    #[allow(unused)]
    shadow_pipeline: ShadowPipeline,
    text_pipeline: TextPipeline,
    circle_pipeline: CirclePipeline,
    line_pipeline: LinePipeline,
    rectangle_pipeline: RectanglePipeline,
    chunk_pipeline: RectanglePipeline,
    sprite_pipeline: SpritePipeline,
}

pub struct RenderState<'a> {
    pub renderer: Renderer<'a>,
    pub window: &'a mut glfw::Window,

    pipelines: Pipelines,

    pub rect_data: RectDataBuffer,
    pub height_data_chunks: BufferResource,

    depth_texture: Texture,
    im1: Texture,
    im2: Texture,
}

impl<'a> RenderState<'a> {
    pub async fn new(window: &'a mut glfw::Window) -> Self {
        let renderer = Renderer::new(window).await;

        let uniform_buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shader Params"),
            size: 40,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let size = window.get_size();

        let depth_texture = Texture::depth_texture(&renderer, "depth_texture");

        let im1 = Texture::blank_texture(&renderer, size.0 as u32, size.1 as u32, "im1");
        let im2 = Texture::blank_texture(&renderer, size.0 as u32, size.1 as u32, "im2");

        let time_etc_data_bind_group =
            renderer
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("uniform_data_bind_group"),
                });

        let rect_data = RectDataBuffer::new(&renderer, RectanglePipeline::RECTS_PER_PASS);

        let (rectangle_pipeline, _) =
            RectanglePipeline::new(&renderer, "crates/rend/shaders/rectangle.wgsl");

        let (chunk_pipeline, height_data_chunks) =
            RectanglePipeline::new(&renderer, "crates/rend/shaders/chunk_terrain.wgsl");

        let sprite_pipeline = SpritePipeline::new(&renderer);

        let uniform_bind_group = {
            let mut builder = BindGroupBuilder::new(&renderer.device);
            builder.set_layout(&time_etc_data_bind_group);
            builder.add_buffer(&uniform_buffer, 0);
            builder.build("uniform buffer")
        };

        let text_pipeline = TextPipeline::new(&renderer);
        let blur_pipeline = BlurPipeline::new(&renderer.device, &renderer.config);
        let circle_pipeline = CirclePipeline::new(&renderer);
        let lava_lamp_pipeline = LavaLampPipeline::new(&renderer);
        let line_pipeline = LinePipeline::new(&renderer);

        let shadow_pipeline = ShadowPipeline::new(&renderer);

        let world = RenderWorld::new();

        let pipelines = Pipelines {
            lava_lamp_pipeline,
            blur_pipeline,
            shadow_pipeline,
            text_pipeline,
            circle_pipeline,
            line_pipeline,
            rectangle_pipeline,
            chunk_pipeline,
            sprite_pipeline,
        };

        Self {
            renderer,
            window,
            pipelines,
            depth_texture,
            im1,
            im2,
            rect_data,
            height_data_chunks,
        }
    }

    pub fn resize(&mut self, new_size: (i32, i32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            let w = new_size.0 as u32;
            let h = new_size.1 as u32;

            self.renderer.config.width = w;
            self.renderer.config.height = h;
            self.renderer
                .surface
                .configure(&self.renderer.device, &self.renderer.config);
            self.depth_texture = Texture::depth_texture(&self.renderer, "depth_texture");

            self.im1 = Texture::blank_texture(&self.renderer, w, h, "im1");
            self.im2 = Texture::blank_texture(&self.renderer, w, h, "im1");
        }
    }

    pub fn update_surface(&mut self) {
        self.renderer.surface = self
            .renderer
            .instance
            .create_surface(self.window.render_context())
            .unwrap();
    }

    #[allow(unused)]
    fn draw_lava(&self, view: &wgpu::TextureView, time: f32) {
        let mut command_encoder = self
            .renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let mut rp = self.get_render_pass(&mut command_encoder, None, &view, true);

        let mouse_pos = self.window.get_cursor_pos();

        let shader_params = ShaderParams {
            mouse: (mouse_pos.0 as f32, mouse_pos.1 as f32),
            time,
            resolution: (
                self.window.get_size().0 as f32,
                self.window.get_size().1 as f32,
            ),
        };

        let transform = mat4_identity();
        self.pipelines.lava_lamp_pipeline.draw(
            &mut rp,
            &transform,
            &shader_params,
            &self.renderer.queue,
        );

        drop(rp);
        self.renderer
            .queue
            .submit(std::iter::once(command_encoder.finish()));
    }

    pub fn clear(&self, view: &wgpu::TextureView, color: Color) {
        let mut command_encoder = self
            .renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let rp = self.get_render_pass(&mut command_encoder, Some(color.to_wgpu()), &view, true);

        drop(rp);

        self.renderer
            .queue
            .submit(std::iter::once(command_encoder.finish()));
    }

    fn draw_circles(
        &self,
        texture: &wgpu::Texture,
        commands: &[CircleCommand],
        new_depth: bool,
    ) -> usize {
        let (sx, sy) = self.window.get_size();

        let view = &texture.create_view(&wgpu::TextureViewDescriptor::default());

        let dims = texture.size();

        let screen = glam::DVec2::new(dims.width as f64, dims.height as f64);

        let mut passes = 0;

        for chunk in commands.chunks(CirclePipeline::MAX_CIRCLES_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view, new_depth);

            self.pipelines
                .circle_pipeline
                .assign_buffer_data(&self.renderer.queue, chunk, screen);

            self.pipelines
                .circle_pipeline
                .draw_circles(&mut rp, chunk.len());

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }

        passes
    }

    fn draw_lines(
        &self,
        view: &wgpu::TextureView,
        commands: &[LineCommand],
        new_depth: bool,
    ) -> usize {
        let (sx, sy) = self.window.get_size();

        let mut passes = 0;

        for chunk in commands.chunks(LinePipeline::MAX_LINES_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view, new_depth);

            self.pipelines.line_pipeline.assign_buffer_data(
                &self.renderer.queue,
                chunk,
                sx as f64,
                sy as f64,
            );

            self.pipelines
                .line_pipeline
                .draw_lines(&mut rp, chunk.len());

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }

        passes
    }

    fn draw_rectangles(
        &self,
        view: &wgpu::TextureView,
        commands: &[RectCommand],
        new_depth: bool,
    ) -> usize {
        let (sx, sy) = self.window.get_size();
        let screen = glam::DVec2::new(sx as f64, sy as f64);

        let mut passes = 0;

        for cmds in commands.chunks(RectanglePipeline::RECTS_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view, new_depth);

            self.rect_data.write(&self.renderer.queue, cmds, screen);

            self.pipelines.rectangle_pipeline.draw(
                &mut rp,
                cmds.len(),
                &[self.rect_data.buffer(), &self.height_data_chunks],
            );

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }

        passes
    }

    fn draw_chunks(
        &self,
        texture: &wgpu::Texture,
        commands: &[ChunkCommand],
        new_depth: bool,
    ) -> usize {
        let view = &texture.create_view(&wgpu::TextureViewDescriptor::default());

        let dims = texture.size();

        let screen = glam::DVec2::new(dims.width as f64, dims.height as f64);

        let mut passes = 0;

        for cmds in commands.chunks(RectanglePipeline::RECTS_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

            let height_data: Vec<u8> = cmds
                .iter()
                .map(|c| c.height.to_vec())
                .flat_map(|c| c.iter().map(|c| c.to_le_bytes()).collect::<Vec<_>>())
                .collect::<Vec<_>>()
                .concat();

            self.height_data_chunks
                .write(&self.renderer.queue, &height_data);

            let rcmds: Vec<_> = cmds
                .iter()
                .map(|c| RectCommand {
                    pos: c.pos,
                    dims: c.dims,
                    angle: c.angle,
                    color: Color::PURPLE.alpha(0.3),
                    z: 0.0,
                })
                .collect();

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view, new_depth);

            self.rect_data.write(&self.renderer.queue, &rcmds, screen);

            self.pipelines.chunk_pipeline.draw(
                &mut rp,
                cmds.len(),
                &[self.rect_data.buffer(), &self.height_data_chunks],
            );

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }

        passes
    }

    fn draw_ui(
        &self,
        world: &RenderWorld,
        view: &wgpu::TextureView,
        font_id: Ent,
        commands: &[CharCommand],
        new_depth: bool,
    ) -> usize {
        let (sx, sy) = self.window.get_size();
        let screen = glam::DVec2::new(sx as f64, sy as f64);

        let (font, material) = world.fonts.get(font_id).unwrap();

        let mut passes = 0;

        for chunk in commands.chunks(TextPipeline::MAX_CHARS_PER_PASS) {
            let mut command_encoder = self
                .renderer
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            passes += 1;

            let mut rp = self.get_render_pass(&mut command_encoder, None, &view, new_depth);

            self.pipelines.text_pipeline.assign_buffer_data(
                &self.renderer.queue,
                chunk,
                font,
                screen,
            );

            self.pipelines
                .text_pipeline
                .draw_text(&mut rp, material, chunk.len());

            drop(rp);

            self.renderer
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }

        passes
    }

    fn get_render_pass<'b>(
        &self,
        command_encoder: &'b mut wgpu::CommandEncoder,
        clear_color: Option<wgpu::Color>,
        view: &wgpu::TextureView,
        clear_depth: bool,
    ) -> wgpu::RenderPass<'b> {
        self.renderer.get_render_pass(
            command_encoder,
            clear_color,
            view,
            &self.depth_texture,
            clear_depth,
        )
    }

    fn copy(&self, incoming: &Texture, outgoing: &wgpu::TextureView) {
        self.blur_pass(incoming, outgoing, false, true, 5.0)
    }

    pub fn blur_pass(
        &self,
        incoming: &Texture,
        outgoing: &wgpu::TextureView,
        is_vertical: bool,
        is_nullopt: bool,
        kernel_size: f64,
    ) {
        let mut command_encoder = self.renderer.make_command_encoder();

        let mut rp = self.get_render_pass(&mut command_encoder, None, &outgoing, true);

        let params = BlurParams {
            resolution: incoming.size.as_vec2(),
            is_vertical,
            is_nullopt,
            kernel_size: kernel_size as f32,
        };

        self.pipelines.blur_pipeline.blur_pass(
            &mut rp,
            &self.renderer.queue,
            params,
            &incoming.bind_group,
        );

        drop(rp);

        self.renderer.submit(command_encoder);
    }

    pub fn apply_geometry_commands(
        &self,
        world: &RenderWorld,
        font_id: Ent,
        layer: &RenderLayer,
        texture: &wgpu::Texture,
    ) -> usize {
        let mut passes = 0;

        let view = &texture.create_view(&wgpu::TextureViewDescriptor::default());

        passes += self.draw_chunks(texture, &layer.chunk_commands, true);
        passes += self.draw_sprites(world, texture, &layer);
        passes += self.draw_rectangles(view, &layer.rect_commands, true);
        passes += self.draw_circles(texture, &layer.circle_commands, true);
        passes += self.draw_lines(view, &layer.line_commands, true);
        passes += self.draw_ui(world, view, font_id, &layer.char_commands, true);

        passes
    }

    fn draw_sprites(
        &self,
        world: &RenderWorld,
        texture: &wgpu::Texture,
        layer: &RenderLayer,
    ) -> usize {
        let (sx, sy) = self.window.get_size();
        let screen = glam::DVec2::new(sx as f64, sy as f64);

        let view = &texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut passes = 0;

        for (sprite_id, rects) in &layer.sprite_commands {
            let texture = world.textures.get(*sprite_id).unwrap();

            for chunk in rects.chunks(RectanglePipeline::RECTS_PER_PASS) {
                passes += 1;

                let mut command_encoder = self
                    .renderer
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                let mut rp = self.get_render_pass(&mut command_encoder, None, view, true);

                self.rect_data.write(&self.renderer.queue, &chunk, screen);

                self.pipelines.sprite_pipeline.draw(
                    &mut rp,
                    &texture.bind_group,
                    &self.rect_data,
                    chunk.len(),
                );

                drop(rp);

                self.renderer
                    .queue
                    .submit(std::iter::once(command_encoder.finish()));
            }
        }

        passes
    }

    pub fn render(
        &mut self,
        world: &RenderWorld,
        commands: &RenderCommands,
    ) -> Result<Option<(SurfaceTexture, usize)>, wgpu::SurfaceError> {
        let (w, h) = self.window.get_size();

        if w == 0 || h == 0 {
            return Ok(None);
        }

        self.renderer.device.poll(wgpu::Maintain::wait());

        {
            let event = self.renderer.queue.submit([]);
            let maintain = wgpu::Maintain::WaitForSubmissionIndex(event);
            self.renderer.device.poll(maintain);
        }

        let drawable = self.renderer.surface.get_current_texture()?;

        let view = drawable
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut passes = 0;

        self.clear(&self.im1.view, Color::rgb(117, 186, 255, 1.0));

        for layer in commands.layers() {
            passes += self.apply_geometry_commands(
                &world,
                commands.current_font_id,
                layer,
                &self.im1.texture,
            );
        }

        self.copy(&self.im1, &view);

        self.renderer.device.poll(wgpu::Maintain::wait());

        Ok(Some((drawable, passes)))
    }
}
