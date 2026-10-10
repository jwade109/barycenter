#![allow(unused)]

use bary_core::prelude::*;
use bary_input::InputState;
use bary_ipc::new_message_queue;
use bary_raylib::*;
use bary_sim::Camera;
use raylib::prelude::*;
use rdev::Key;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum LegState {
    Grappled,
    Travelling,
    Retracted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ExtensionState {
    Overextended,
    Nominal,
    UnderExtended,
}

const LEG_RADIUS: f64 = 2.0;
const MIN_LEG_LENGTH: f64 = 0.6;
const MAX_LEG_LENGTH: f64 = 3.0;

struct Leg {
    min_length: f64,
    max_length: f64,
    desired_angle: f64,
    foot_position: DVec2,
    target_foot_position: DVec2,
    state: LegState,
}

impl Leg {
    fn desired_length(&self) -> f64 {
        (self.max_length + self.min_length) / 2.0
    }

    fn desired_offset(&self, spider_angle: f64) -> DVec2 {
        rotate_f64(DVec2::X, self.desired_angle + spider_angle) * self.desired_length()
    }

    fn desired_foot_pos(&self, spider: Isometry2d) -> DVec2 {
        self.desired_offset(spider.rotation) + spider.translation
    }

    fn should_replant(&self, spider: Isometry2d) -> bool {
        let p = self.desired_foot_pos(spider);
        let a = self.foot_position;
        let dist_from_desired = p.distance(a);
        dist_from_desired > MIN_LEG_LENGTH
            || self.extension_state(spider.translation) != ExtensionState::Nominal
    }

    fn update(&mut self, spider: Isometry2d, dt: f64) {
        if self.state == LegState::Travelling {
            let max_speed = 65.0;
            let max_delta = max_speed * dt;
            let u = self.target_foot_position - self.foot_position;
            let delta = max_delta.min(u.length());
            let u = u.normalize_or_zero();
            let delta = u * delta;
            self.foot_position += delta;
            if delta.length() < 0.01 {
                self.state = LegState::Grappled;
            }
        }
    }

    fn replant(&mut self, spider: Isometry2d) {
        if self.state == LegState::Retracted {
            return;
        }
        let c = self.foot_position;
        let p = self.desired_foot_pos(spider);
        let u = p - c;
        let u = u.normalize_or_zero() * u.length().clamp(0.0, MIN_LEG_LENGTH);
        self.target_foot_position = p + u * 0.7; // vround(p / 2.0).as_vec2() * 2.0;
        self.state = LegState::Travelling;
    }

    fn extension_state(&self, spider: DVec2) -> ExtensionState {
        let l = spider.distance(self.foot_position);
        if l < self.min_length {
            ExtensionState::UnderExtended
        } else if l > self.max_length {
            ExtensionState::Overextended
        } else {
            ExtensionState::Nominal
        }
    }
}

struct Spider {
    pose: Isometry2d,
    vel: Isometry2d,
    legs: Vec<Leg>,
    mass: f32,
    is_drifting: bool,
}

fn make_legs(n: usize) -> Vec<Leg> {
    let angle_offset = bary_core::prelude::PI_64 / n as f64;

    (0..n)
        .map(|i| {
            let a = angle_offset + i as f64 / n as f64 * 2.0 * bary_core::prelude::PI_64;
            Leg {
                desired_angle: a,
                min_length: MIN_LEG_LENGTH,
                max_length: MAX_LEG_LENGTH,
                foot_position: DVec2::ZERO,
                target_foot_position: rotate_f64(DVec2::X, a) * 2.0,
                state: LegState::Travelling,
            }
        })
        .collect()
}

impl Spider {
    fn new(angle: f64, n_legs: usize) -> Self {
        Self {
            pose: Isometry2d::from_xya(0.0, 0.0, angle),
            vel: Isometry2d::ZERO,
            legs: make_legs(n_legs),
            mass: 1.0,
            is_drifting: false,
        }
    }

    fn toggle_drifting(&mut self) {
        let new_state = if self.is_drifting {
            LegState::Travelling
        } else {
            LegState::Retracted
        };

        for leg in &mut self.legs {
            leg.state = new_state;
        }

        self.is_drifting ^= true;

        if !self.is_drifting {
            for leg in &mut self.legs {
                leg.foot_position = self.pose.translation;
                leg.target_foot_position = leg.desired_foot_pos(self.pose);
                leg.state = LegState::Travelling;
            }
        }
    }

    fn worst_leg(&self) -> Option<usize> {
        self.legs
            .iter()
            .enumerate()
            .filter_map(|(i, l)| {
                let d = l.desired_foot_pos(self.pose);
                let p = l.foot_position;
                let d = d.distance(p);
                (l.should_replant(self.pose)).then(|| (i, d))
            })
            .max_by(|x, y| x.1.total_cmp(&y.1))
            .map(|e| e.0)
    }
}

struct World {
    time: f64,
    spiders: Vec<Spider>,
    camera: Camera,
}

fn draw_spider(d: &mut RaylibDrawHandle, spider: &Spider) {
    for leg in &spider.legs {
        if leg.state == LegState::Retracted {
            continue;
        }
        let offset = leg.desired_offset(spider.pose.rotation);
        let s = spider.pose.translation;
        let e = leg.foot_position;

        let color = if leg.state == LegState::Travelling {
            Color::GRAY
        } else {
            Color::ORANGE
        };

        draw_line_width(d, s, e, 0.2, color);
        fill_circle(d, e, 0.2, color);
        draw_circle(d, s, MIN_LEG_LENGTH, Color::TEAL);
        draw_circle(d, s, MAX_LEG_LENGTH, Color::TEAL);

        if leg.state == LegState::Travelling {
            let p = leg.target_foot_position;
            // fill_circle(d, p, 0.15, Color::YELLOW);
        }
    }

    let body_dims = Vec2::new(1.1, 1.6);
    let mut body_iso = spider.pose;
    body_iso = body_iso.offset(-body_dims / 2.0);
    fill_rectangle(d, body_iso, body_dims, Color::TEAL);
}

fn draw_world(handle: &mut RaylibDrawHandle, world: &World) {
    let screen_dims = Vec2::new(
        handle.get_screen_width() as f32,
        handle.get_screen_height() as f32,
    );

    let cam = to_raylib_camera(&world.camera, screen_dims);
    let mut draw_handle = handle.begin_mode2D(cam);
    let d = &mut draw_handle;

    for x in (-20..=20).step_by(2) {
        for y in (-20..=20).step_by(2) {
            fill_circle(d, Vec2::new(x as f32, y as f32), 0.05, Color::RED);
        }
    }

    for x in (-100..100).step_by(10) {
        let s = Vec2::new(x as f32, -1000.0);
        let e = Vec2::new(x as f32, 1000.0);
        draw_line(d, s, e, Color::GRAY);
        let s = Vec2::new(-1000.0, x as f32);
        let e = Vec2::new(1000.0, x as f32);
        draw_line(d, s, e, Color::GRAY);
    }

    for spider in &world.spiders {
        draw_spider(d, spider);
    }
}

fn update_spider(spider: &mut Spider, dt: f64) {
    let mut dv = Vec2::ZERO;

    let n_moving = spider
        .legs
        .iter()
        .filter(|l| l.state == LegState::Travelling)
        .count();

    if n_moving < 2 {
        if let Some(i) = spider.worst_leg() {
            spider.legs[i].replant(spider.pose);
        }
    }

    for leg in &mut spider.legs {
        leg.update(spider.pose, dt);
    }

    if spider.vel.translation.length() > 0.0 {
        let angle = spider.vel.translation.to_angle();
        let angle_vel = (angle - spider.pose.rotation) * 10.0;
        spider.vel.rotation = angle_vel;
    } else {
        spider.vel.rotation = 0.0;
    }

    spider.pose.translation += spider.vel.translation * dt;
    spider.pose.rotation += spider.vel.rotation * dt;
}

fn update_world(world: &mut World, dt: f64) {
    world.time += dt;

    for spider in &mut world.spiders {
        update_spider(spider, dt);
    }

    let vel = DVec2::new(world.time.cos(), world.time.sin()) * 10.0;

    world.spiders[1].vel.translation = vel;

    world.camera.isometry.translation +=
        (world.spiders[0].pose.translation - world.camera.isometry.translation) * 1.0;
}

fn make_world() -> World {
    World {
        time: 0.0,
        spiders: vec![Spider::new(0.4, 6), Spider::new(0.0, 3)],
        camera: Camera {
            isometry: Isometry2d::ZERO,
            zoom: 70.0,
        },
    }
}

fn process_input(world: &mut World, input: &InputState) {
    let n = input.is_key_pressed(Key::KeyW);
    let w = input.is_key_pressed(Key::KeyA);
    let s = input.is_key_pressed(Key::KeyS);
    let e = input.is_key_pressed(Key::KeyD);

    let l = input.is_key_pressed(Key::KeyQ);
    let r = input.is_key_pressed(Key::KeyE);

    let x_pull = -(w as i8) + e as i8;
    let y_pull = -(s as i8) + n as i8;

    let speed = 16.0;

    let dir = DVec2::new(x_pull as f64, y_pull as f64).normalize_or_zero();

    let vel = dir * speed;

    let mut spider = &mut world.spiders[0];

    let rot_dir = l as i8 - r as i8;

    spider.vel.translation = vel;

    if input.just_pressed_debounced(Key::Space) {
        spider.toggle_drifting();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut handle, thread) = raylib::init()
        .size(1080, 700)
        .title("Spider Demo")
        .log_level(TraceLogLevel::LOG_WARNING)
        .msaa_4x()
        .resizable()
        .build();

    handle.set_target_fps(120);
    handle.maximize_window();

    let mut input = InputState::default();

    let input_queue = new_message_queue();
    let thread_copy = input_queue.clone();

    let _input_thread = std::thread::spawn(|| {
        if let Err(error) = rdev::listen(move |e| thread_copy.push(e)) {
            println!("Error: {:?}", error)
        }
    });

    let mut world = make_world();

    while !handle.window_should_close() {
        let dt = handle.get_frame_time() as f64;

        while let Some(event) = input_queue.pop() {
            input.process_rdev_event(&event, handle.is_window_focused());
        }

        process_input(&mut world, &input);
        update_world(&mut world, dt);

        handle.draw(&thread, |mut d| {
            d.clear_background(Color::BLACK);
            draw_world(&mut d, &world);
        });

        if input.is_key_pressed(Key::Escape) {
            break;
        }

        if input.is_key_pressed(Key::Minus) {
            world.camera.zoom /= 1.03;
            world.camera.zoom = world.camera.zoom.clamp(1.0, 100.0);
        }

        if input.is_key_pressed(Key::Equal) {
            world.camera.zoom *= 1.03;
            world.camera.zoom = world.camera.zoom.clamp(1.0, 100.0);
        }

        input.on_frame_boundary();
    }

    Ok(())
}
