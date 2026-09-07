#![allow(unused)]

use crate::draw::draw_world;
use crate::event_bus::*;
use crate::rend_app::*;
use crate::render_state::RenderState;
use crate::tweens::AnimationStates;
use crate::world::*;
use bary_core::prelude::*;
use bary_input::InputState;
use bary_ipc::{MessageQueue, new_message_queue};
use glfw::*;
use log::*;
use rend::*;
use sounds::*;
use std::{
    collections::BTreeMap,
    thread::JoinHandle,
    time::{Duration, Instant},
};

mod bezier;
mod draw;
mod event_bus;
mod node;
mod persistence;
mod railcar;
mod rend_app;
mod render_state;
mod render_world;
mod sounds;
mod terrain;
mod track;
mod tweens;
mod viewport;
mod world;

struct TrainApp<'a> {
    last: Instant,
    frame_timer: WallTimer,
    timers: BTreeMap<&'static str, Duration>,
    draw_calls: usize,
    world: World,
    events: EventBus,
    selection: SelectionInfo,
    animations: AnimationStates,
    input_state: InputState,
    rs: RenderState<'a>,
    _input_thread: JoinHandle<()>,
    input_queue: MessageQueue<rdev::Event>,
    should_exit: bool,
    sounds: SoundManager,

    inv_id: Ent,
    mush_id: Ent,
}

impl<'a> TrainApp<'a> {
    async fn new(window: &'a mut glfw::Window) -> Self {
        let mut rs = RenderState::new(window).await;
        let input_queue = new_message_queue();
        let thread_copy = input_queue.clone();

        let _input_thread = std::thread::spawn(|| {
            if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
                error!("Error: {:?}", error)
            }
        });

        let font_id = rs.world.load_font(&rs.renderer, "consolas_spritesheet");
        rs.world.load_font(&rs.renderer, "cambria");
        rs.world.load_font(&rs.renderer, "garamond");
        rs.world.load_font(&rs.renderer, "arial");
        rs.world.load_font(&rs.renderer, "calibri");
        rs.world.load_font(&rs.renderer, "verdana");
        rs.world.load_font(&rs.renderer, "impact");
        rs.world.load_font(&rs.renderer, "courier_new");

        let inv_id = rs.world.load_texture(&rs.renderer, "assets/invincible.jpg");
        let mush_id = rs.world.load_texture(&rs.renderer, "assets/mushroom.jpg");

        rs.window.set_framebuffer_size_polling(true);
        rs.window.set_key_polling(true);
        rs.window.set_mouse_button_polling(true);
        rs.window.set_pos_polling(true);

        let mut events = EventBus::new();

        let world = make_world(&mut events, font_id, inv_id, mush_id);

        Self {
            frame_timer: WallTimer::hz(10000),
            last: Instant::now(),
            rs,
            timers: BTreeMap::new(),
            draw_calls: 0,
            world,
            events,
            selection: SelectionInfo::new(),
            animations: AnimationStates::new(),
            input_state: InputState::default(),
            _input_thread,
            input_queue,
            should_exit: false,
            sounds: SoundManager::new(),

            inv_id,
            mush_id,
        }
    }
}

impl<'a> RendApp for TrainApp<'a> {
    fn update(&mut self) {
        self.frame_timer.tick();

        let start = Instant::now();

        self.input_state.on_frame_boundary();

        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64();

        self.animations.update(dt);
        while let Some(event) = self.input_queue.pop() {
            self.input_state
                .process_rdev_event(&event, self.rs.window.is_focused());
        }

        let (width, height) = self.rs.window.get_size();
        let screen_width = DVec2::new(width as f64, height as f64);
        let mouse = DVec2::new(
            self.rs.window.get_cursor_pos().0,
            self.rs.window.get_cursor_pos().1,
        );

        let mouse = mouse.with_y(height as f64 - mouse.y);

        update_world(
            &mut self.world,
            &mut self.selection,
            dt as f64,
            mouse,
            screen_width,
        );

        process_input(
            &mut self.world,
            &mut self.rs,
            &mut self.events,
            &mut self.selection,
            &self.input_state,
            dt as f64,
            mouse,
            screen_width,
        );

        self.sounds.update();

        if self.input_state.is_key_pressed(rdev::Key::Escape) {
            self.should_exit = true;
        }

        self.rs
            .world
            .handle_events(&self.rs.renderer, &self.events, &mut self.world);

        self.sounds.handle_events(&self.events);

        self.events.clear();

        self.last = now;

        self.timers.insert("update", Instant::now() - start);
    }

    fn emit_render_commands(&mut self) -> RenderCommands {
        let start = Instant::now();

        let mut cmd = RenderCommands::from_fonts(&self.rs.world.fonts);
        cmd.current_font_id = self.world.current_font_id.unwrap();
        let (width, height) = self.rs.window.get_size();
        let dims = DVec2::new(width as f64, height as f64);
        let mouse = DVec2::new(
            self.rs.window.get_cursor_pos().0,
            self.rs.window.get_cursor_pos().1,
        );

        let mouse = mouse.with_y(height as f64 - mouse.y);
        let mut font = FontSelection::new();

        draw_world(
            &mut cmd,
            &self.selection,
            &mut self.events,
            &mut font,
            &self.input_state,
            &self.world,
            dims,
            mouse,
            &self.animations,
            self.draw_calls,
            &self.frame_timer,
            &self.timers,
        );

        if let Some(font_id) = font.new_font_id() {
            self.world.current_font_id = Some(font_id);
        }

        self.timers.insert("commands", Instant::now() - start);

        cmd
    }

    fn on_event(&mut self, _event: &glfw::WindowEvent) {}

    fn render(&mut self, commands: &RenderCommands) {
        let start = Instant::now();
        let render = self.rs.render(&commands);
        self.timers.insert("render", Instant::now() - start);

        let start = Instant::now();
        match render {
            Ok(Some((drawable, count))) => {
                drawable.present();
                self.draw_calls = count;
            }
            Ok(None) => (),
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                self.rs.update_surface();
                self.rs.resize(self.rs.window.get_size());
            }
            Err(e) => error!("{:?}", e),
        }
        self.timers.insert("present", Instant::now() - start);
    }

    fn should_close(&self) -> bool {
        // todo!()
        self.rs.window.should_close() || self.should_exit
    }
}

fn run(
    mut glfw: glfw::Glfw,
    events: glfw::GlfwReceiver<(f64, glfw::WindowEvent)>,
    mut app: impl RendApp,
) {
    while !app.should_close() {
        glfw.poll_events();

        for (_, event) in glfw::flush_messages(&events) {
            app.on_event(&event);
        }

        app.update();

        let commands = app.emit_render_commands();
        app.render(&commands);
    }
}

async fn init() {
    let mut glfw = glfw::init(fail_on_errors!()).unwrap();
    glfw.window_hint(WindowHint::ClientApi(ClientApiHint::NoApi));
    let (mut window, events) = glfw
        .create_window(1200, 950, "It's WGPU time.", glfw::WindowMode::Windowed)
        .unwrap();

    window.maximize();

    run(glfw, events, TrainApp::new(&mut window).await);
}

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();

    pollster::block_on(init());
}
