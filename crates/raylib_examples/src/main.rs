/*******************************************************************************************
 *
 *   raylib [shaders] example - texture waves
 *
 *   Example complexity rating: [★★☆☆] 2/4
 *
 *   NOTE: This example requires raylib OpenGL 3.3 or ES2 versions for shaders support,
 *         OpenGL 1.1 does not support shaders, recompile raylib to OpenGL 3.3 version
 *
 *   NOTE: Shaders used in this example are #version 330 (OpenGL 3.3), to test this example
 *         on OpenGL ES 2.0 platforms (Android, Raspberry Pi, HTML5), use #version 100 shaders
 *         raylib comes with shaders ready for both versions, check raylib/shaders install folder
 *
 *   Example originally created with raylib 2.5, last time updated with raylib 3.7
 *
 *   Example contributed by Anata (@anatagawa) and reviewed by Ramon Santamaria (@raysan5)
 *
 *   Example licensed under an unmodified zlib/libpng license, which is an OSI-certified,
 *   BSD-like license that allows static linking with closed source software
 *
 *   Copyright (c) 2019-2025 Anata (@anatagawa) and Ramon Santamaria (@raysan5)
 *
 ********************************************************************************************/

use std::time::{Duration, Instant};

use raylib::core::shaders::RaylibShader;
use raylib::prelude::*;

fn set_shader_value<T: ShaderV>(shader: &mut Shader, name: &str, value: T) {
    let loc = shader.get_shader_location(name);
    shader.set_shader_value(loc, value);
}

fn main() {
    let (mut rl, thread) = raylib::init()
        .title("raylib [shaders] example - texture waves")
        .resizable()
        .build();

    let texture_path = "assets/space.png";
    let shader_path = "assets/shaders/thruster.fs";

    // Load texture texture to apply shaders
    let texture = rl.load_texture(&thread, texture_path).unwrap();

    // Load shader and setup location points and values
    let mut shader = rl.load_shader(&thread, None, Some(shader_path));

    if !shader.is_shader_valid() {
        println!("Invalid shader!");
        return;
    }

    let mut seconds = 0.0f32;

    rl.set_target_fps(60); // Set our game to run at 60 frames-per-second

    let mut last_loaded = Instant::now();

    // Main game loop
    while !rl.window_should_close() {
        let now = Instant::now();
        let should_reload = now - last_loaded > Duration::from_millis(100);
        if should_reload {
            last_loaded = now;
            println!("Trying to reload shader");
            let new_shader = rl.load_shader(&thread, None, Some(shader_path));
            if new_shader.is_shader_valid() {
                println!("Shader is valid");
                shader = new_shader;
            } else {
                println!("Shader error!");
            }
        }

        seconds += rl.get_frame_time();

        let screen_size = [rl.get_screen_width() as f32, rl.get_screen_height() as f32];

        set_shader_value(&mut shader, "_current_time", seconds);
        set_shader_value(&mut shader, "_resolution", screen_size);

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::PURPLE);

        let source_rec = Rectangle::new(0.0, 0.0, texture.width() as f32, texture.height() as f32);

        let dest_rec = Rectangle::new(
            0.0,
            0.0,
            d.get_render_width() as f32,
            d.get_render_height() as f32,
        );

        d.draw_texture_pro(
            &texture,
            source_rec,
            dest_rec,
            Vector2::zero(),
            0.0,
            Color::WHITE,
        );

        {
            let mut s = d.begin_shader_mode(&mut shader);

            s.draw_rectangle(
                0,
                0,
                s.get_render_width(),
                s.get_render_height(),
                Color::WHITE,
            );
        }
    }
}
