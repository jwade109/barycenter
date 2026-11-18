use std::{
    rc::Rc,
    time::{Duration, Instant},
};

use raylib::{
    color::Color,
    drawing::{RaylibDraw, RaylibDrawHandle},
    math::Vector2,
    text::{Font, RaylibFont},
};

pub struct AppearingText {
    font: Rc<Font>,
    message: String,
    start: Instant,
}

const DURATION_PER_CHAR: Duration = Duration::from_millis(20);
const LOITER_TIME: Duration = Duration::from_secs(5);

impl AppearingText {
    pub fn new(message: impl Into<String>, font: Rc<Font>) -> Self {
        Self {
            font,
            message: message.into(),
            start: Instant::now(),
        }
    }

    fn n_chars_visible(&self) -> usize {
        let tick = DURATION_PER_CHAR.as_secs_f64();
        let now = Instant::now();
        ((now - self.start).as_secs_f64() / tick).floor() as usize
    }

    fn age(&self) -> Duration {
        Instant::now() - self.start
    }

    pub fn visible(&self) -> String {
        let n = self.n_chars_visible();
        self.message.chars().take(n).collect()
    }

    fn animation_time(&self) -> Duration {
        DURATION_PER_CHAR * self.message.len() as u32
    }

    pub fn should_remain(&self) -> bool {
        self.alpha() > 0.0 && self.age() > Duration::ZERO
    }

    fn alpha(&self) -> f32 {
        let age = self.age().as_secs_f32();
        let max_age = self.animation_time() + LOITER_TIME;
        let down_ramp = max_age.as_secs_f32() - age;
        let up_ramp = age * 4.0;
        down_ramp.min(up_ramp.min(1.0))
    }

    pub fn draw(&self, rl: &mut RaylibDrawHandle) {
        let width = rl.get_render_width();
        let height = rl.get_render_height();

        let screen_center = Vector2::new(width as f32, height as f32) / 2.0;

        let spacing = self.age().as_secs_f32() / 10.0;

        let font_size = 28.0;
        let text = self.visible();
        let meas = self.font.measure_text(&self.message, font_size, spacing);

        let position = screen_center - meas / 2.0;

        rl.draw_text_ex(
            self.font.as_ref(),
            &text,
            position,
            font_size,
            spacing,
            Color::WHITE.alpha(self.alpha()),
        );
    }
}
