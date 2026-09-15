use crate::{Color, FontInfo, Texture, TextureHandle};
use bary_core::prelude::{rotate_f64, Components, Ent, Isometry2d};
use glam::{DVec2, DVec3, IVec2};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum RenderCommand {
    Char(CharCommand),
    Rect(RectCommand),
    Circle(CircleCommand),
    Line(LineCommand),
    Chunk(ChunkCommand),
    Sprite(Ent, RectCommand),
}

#[derive(Debug, Clone, Copy)]
pub struct RectCommand {
    pub pos: DVec2,
    pub z: f64,
    pub dims: DVec2,
    pub angle: f64,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct CharCommand {
    pub pos: DVec3,
    pub dims: DVec2,
    pub angle: f64,
    pub c: char,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct CircleCommand {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct LineCommand {
    pub start: DVec2,
    pub end: DVec2,
    pub thickness: f64,
    pub color: Color,
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkCommand {
    pub pos: DVec2,
    pub dims: DVec2,
    pub angle: f64,
    pub height: [f32; 4],
}

#[derive(Debug, Default, Clone)]
pub struct RenderLayer {
    pub name: &'static str,
    pub rect_commands: Vec<RectCommand>,
    pub char_commands: Vec<CharCommand>,
    pub circle_commands: Vec<CircleCommand>,
    pub line_commands: Vec<LineCommand>,
    pub chunk_commands: Vec<ChunkCommand>,
    pub sprite_commands: BTreeMap<Ent, Vec<RectCommand>>,
}

impl RenderLayer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

pub struct RenderCommands {
    layers: Vec<RenderLayer>,
    current_layer: usize,
    pub fonts: Components<FontInfo>,
    pub current_font_id: Ent,
}

impl RenderCommands {
    pub fn from_fonts(fonts: &Components<(FontInfo, Texture)>) -> Self {
        let mut font_info = Components::default();

        for (id, (info, _tex)) in fonts.iter() {
            font_info.spawn(*id, info.clone());
        }

        Self::new(font_info)
    }

    fn new(fonts: Components<FontInfo>) -> Self {
        let (id, _) = fonts.iter().next().unwrap();
        let id = *id;
        Self {
            fonts,
            layers: vec![RenderLayer::default()],
            current_layer: 0,
            current_font_id: id,
        }
    }

    pub fn clear(&mut self) {
        self.layers = vec![RenderLayer::default()];
        self.current_layer = 0;
    }

    fn current_layer(&mut self) -> &mut RenderLayer {
        self.layers.get_mut(self.current_layer).unwrap()
    }

    pub fn layers(&self) -> impl Iterator<Item = &RenderLayer> {
        self.layers.iter()
    }

    pub fn new_layer(&mut self, name: &'static str) {
        self.layers.push(RenderLayer::default());
        self.current_layer += 1;
    }

    pub fn enqueue(&mut self, command: RenderCommand) {
        let layer = self.current_layer();
        match command {
            RenderCommand::Char(c) => layer.char_commands.push(c),
            RenderCommand::Rect(c) => layer.rect_commands.push(c),
            RenderCommand::Circle(c) => layer.circle_commands.push(c),
            RenderCommand::Line(c) => layer.line_commands.push(c),
            RenderCommand::Chunk(c) => layer.chunk_commands.push(c),
            RenderCommand::Sprite(id, rect) => {
                layer
                    .sprite_commands
                    .entry(id)
                    .and_modify(|v| v.push(rect))
                    .or_insert(vec![rect]);
            }
        }
    }

    pub fn rect(&mut self, p: impl Into<Isometry2d>) -> RectBuilder<'_> {
        let p = p.into();
        RectBuilder::new(self, p)
    }

    pub fn circle(&mut self, p: impl Into<DVec2>) -> CircleBuilder<'_> {
        let p = p.into();
        let builder: CircleBuilder<'_> = CircleBuilder::new(self, p.x, p.y);
        builder
    }

    pub fn line(&mut self, start: impl Into<DVec2>, end: impl Into<DVec2>) -> LineBuilder<'_> {
        let builder = LineBuilder::new(self, start.into(), end.into());
        builder
    }

    pub fn linestring(&mut self, points: Vec<DVec2>) -> LineStringBuilder<'_> {
        let builder = LineStringBuilder::new(self, points);
        builder
    }

    pub fn isometry(&mut self, iso: impl Into<Isometry2d>, length: f64) {
        let iso = iso.into();
        let c = iso.translation.as_dvec2();
        let x = c + rotate_f64(DVec2::X, iso.rotation as f64) * length;
        let y = c + rotate_f64(DVec2::Y, iso.rotation as f64) * length;

        self.line(c, x).thickness(6.0).color(Color::RED);
        self.line(c, y).thickness(6.0).color(Color::GREEN);
    }

    pub fn text(&mut self, iso: impl Into<Isometry2d>, text: impl AsRef<str>) -> TextBuilder<'_> {
        let iso = iso.into();
        let font = self.fonts.get(self.current_font_id).unwrap().clone();
        TextBuilder::new(self, iso, &font, self.current_font_id, text.as_ref())
    }

    pub fn text_with_shadow(
        &mut self,
        iso: impl Into<Isometry2d>,
        offset: impl Into<DVec2>,
        text: impl AsRef<str>,
        font_size: f64,
        color: Color,
        shadow_color: Color,
    ) {
        let iso = iso.into();
        let shadow = iso.offset(offset);
        self.text(shadow, &text).size(font_size).color(shadow_color);
        self.text(iso, &text).size(font_size).color(color);
    }

    pub fn frame(
        &mut self,
        iso: impl Into<Isometry2d>,
        dims: impl Into<DVec2>,
    ) -> LineStringBuilder<'_> {
        let iso = iso.into();
        let dims = dims.into();

        let a = iso.tr();
        let b = iso.offset(DVec2::X * dims.x).tr();
        let c = iso.offset(dims).tr();
        let d = iso.offset(DVec2::Y * dims.y).tr();

        self.linestring(vec![a, b, c, d, a])
    }

    pub fn sprite<'a>(
        &'a mut self,
        handle: TextureHandle,
        iso: impl Into<Isometry2d>,
    ) -> RectBuilder<'a> {
        RectBuilder::new(self, iso)
            .dims(handle.size.as_dvec2())
            .sprite(handle.id)
            .color(Color::WHITE)
    }

    pub fn chunk(&mut self, iso: impl Into<Isometry2d>, dims: impl Into<DVec2>, height: [f32; 4]) {
        let iso = iso.into();
        let c = ChunkCommand {
            pos: iso.tr(),
            dims: dims.into(),
            angle: iso.rotation as f64,
            height,
        };

        self.enqueue(RenderCommand::Chunk(c));
    }
}

fn generate_text_layout(
    iso: Isometry2d,
    font_id: Ent,
    font: &FontInfo,
    font_size: f64,
    text: &str,
    color: Color,
    layout_width: Option<f64>,
    z: f64,
) -> (Vec<CharCommand>, DVec2) {
    let right = iso.local_x().as_dvec2();
    let down = -iso.local_y().as_dvec2();

    let mut col_offset = 0;
    let mut row = 0;

    let mut cursor = iso.translation.as_dvec2() + down * font_size;

    let font_size = font_size / font.size as f64;

    let mut char_commands = Vec::new();
    let mut sum_x = 0.0;
    let mut max_sum_x: f64 = 0.0;

    for ch in text.chars() {
        if ch == '\n' {
            row += 1;
            cursor =
                iso.translation.as_dvec2() + down * (row + 1) as f64 * font.size as f64 * font_size;
            col_offset = 0;
            sum_x = 0.0;
            continue;
        }

        let Some(data) = font.characters.get(&ch) else {
            continue;
        };

        // if ch == ' ' && col_offset == 0 {
        //     continue;
        // }

        if ch != ' ' {
            let dims = DVec2::new(data.width as f64, data.height as f64) * font_size;
            let yoff = (data.origin_y as f64 - data.height as f64) * font_size;
            let xoff = data.origin_x as f64 * font_size;
            let pos = cursor - yoff * down - xoff * right;
            let pos = DVec3::new(pos.x, pos.y, z);
            char_commands.push(CharCommand {
                pos,
                dims,
                c: ch,
                color,
                angle: iso.rotation as f64,
            });
        }

        col_offset += 1;

        let delta_x = data.advance as f64 * font_size;
        cursor += right * delta_x;
        sum_x += delta_x;

        max_sum_x = max_sum_x.max(sum_x);

        if let Some(layout_width) = layout_width {
            if ch == ' ' && sum_x > layout_width {
                row += 1;
                cursor = iso.translation.as_dvec2() + down * (row + 1) as f64;
                col_offset = 0;
                sum_x = 0.0;
            }
        }
    }

    let extent = DVec2::new(max_sum_x, (row + 1) as f64 * (font.size as f64 * font_size));

    (char_commands, extent)
}

pub struct TextBuilder<'a> {
    commands: &'a mut RenderCommands,
    iso: Isometry2d,
    z: f64,
    font_id: Ent,
    font: FontInfo,
    font_size: f64,
    text: String,
    color: Color,
    layout_width: Option<f64>,
}

impl<'a> TextBuilder<'a> {
    pub fn new(
        commands: &'a mut RenderCommands,
        iso: impl Into<Isometry2d>,
        font: &FontInfo,
        font_id: Ent,
        text: &str,
    ) -> TextBuilder<'a> {
        Self {
            commands,
            iso: iso.into(),
            z: 0.1,
            font_id,
            font: font.clone(),
            font_size: 32.0,
            text: text.into(),
            color: Color::WHITE,
            layout_width: None,
        }
    }

    fn build(&self) -> (Vec<CharCommand>, DVec2) {
        generate_text_layout(
            self.iso,
            self.font_id,
            &self.font,
            self.font_size,
            &self.text,
            self.color,
            self.layout_width,
            self.z,
        )
    }

    pub fn size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn extent(&self) -> DVec2 {
        self.build().1
    }

    pub fn z(mut self, z: f64) -> Self {
        self.z = z;
        self
    }
}

impl<'a> Drop for TextBuilder<'a> {
    fn drop(&mut self) {
        let (c, _) = self.build();
        let layer = self.commands.current_layer();
        layer.char_commands.extend(c);
    }
}

pub struct CircleBuilder<'a> {
    commands: &'a mut RenderCommands,
    x: f64,
    y: f64,
    inner_radius: f64,
    outer_radius: f64,
    color: Color,
    z: f64,
}

impl<'a> CircleBuilder<'a> {
    fn new(commands: &'a mut RenderCommands, x: f64, y: f64) -> Self {
        Self {
            commands,
            x,
            y,
            inner_radius: -4.0,
            outer_radius: 25.0,
            color: Color::new(0.0, 0.3, 1.0, 0.8),
            z: 0.3,
        }
    }

    pub fn radius(mut self, radius: f64) -> Self {
        self.outer_radius = radius;
        self
    }

    pub fn inner_radius(mut self, radius: f64) -> Self {
        self.inner_radius = radius;
        self
    }

    pub fn radii(mut self, inner: f64, outer: f64) -> Self {
        self.inner_radius = inner;
        self.outer_radius = outer;
        self
    }

    pub fn diameter(mut self, diameter: f64) -> Self {
        self.outer_radius = diameter / 2.0;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn z(mut self, z: f64) -> Self {
        self.z = z;
        self
    }
}

impl<'a> Drop for CircleBuilder<'a> {
    fn drop(&mut self) {
        let circle = CircleCommand {
            x: self.x,
            y: self.y,
            z: self.z,
            inner_radius: self.inner_radius,
            outer_radius: self.outer_radius,
            color: self.color,
        };

        if self.inner_radius < self.outer_radius && self.outer_radius > 0.0 {
            self.commands.enqueue(RenderCommand::Circle(circle));
        }
    }
}

pub struct LineBuilder<'a> {
    commands: &'a mut RenderCommands,
    start: DVec2,
    end: DVec2,
    thickness: f64,
    color: Color,
}

impl<'a> LineBuilder<'a> {
    fn new(commands: &'a mut RenderCommands, start: DVec2, end: DVec2) -> Self {
        Self {
            commands,
            start,
            end,
            thickness: 10.0,
            color: Color::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'a> Drop for LineBuilder<'a> {
    fn drop(&mut self) {
        let line = LineCommand {
            start: self.start,
            end: self.end,
            thickness: self.thickness,
            color: self.color,
        };
        self.commands.enqueue(RenderCommand::Line(line));
    }
}

pub struct RectBuilder<'a> {
    commands: &'a mut RenderCommands,
    pos: DVec2,
    dims: DVec2,
    angle: f64,
    z: f64,
    color: Color,
    centered: bool,
    sprite_id: Option<Ent>,
}

impl<'a> RectBuilder<'a> {
    pub fn new(commands: &'a mut RenderCommands, iso: impl Into<Isometry2d>) -> Self {
        let iso = iso.into();
        Self {
            commands,
            pos: iso.translation.as_dvec2(),
            dims: DVec2::splat(70.0),
            angle: iso.rotation as f64,
            z: 0.5,
            color: Color::new(0.2, 1.0, 0.2, 1.0),
            centered: false,
            sprite_id: None,
        }
    }

    pub fn dims(mut self, dims: impl Into<DVec2>) -> Self {
        self.dims = dims.into();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn angle(mut self, angle: f64) -> Self {
        self.angle = angle;
        self
    }

    pub fn centered(mut self) -> Self {
        self.centered = true;
        self
    }

    pub fn sprite(mut self, id: Ent) -> Self {
        self.sprite_id = Some(id);
        self
    }

    pub fn z(mut self, z: f64) -> Self {
        self.z = z;
        self
    }
}

impl<'a> Drop for RectBuilder<'a> {
    fn drop(&mut self) {
        let pos = if self.centered {
            let iso = Isometry2d::ZERO.with_rotation(self.angle as f32);
            let x = iso.local_x().as_dvec2();
            let y = iso.local_y().as_dvec2();
            let d = self.dims / 2.0;
            let off = x * d.x + y * d.y;
            self.pos - off
        } else {
            self.pos
        };

        let cmd = RectCommand {
            pos,
            dims: self.dims,
            angle: self.angle,
            color: self.color,
            z: self.z,
        };

        if let Some(id) = self.sprite_id {
            self.commands.enqueue(RenderCommand::Sprite(id, cmd));
        } else {
            self.commands.enqueue(RenderCommand::Rect(cmd));
        }
    }
}

pub struct LineStringBuilder<'a> {
    commands: &'a mut RenderCommands,
    points: Vec<DVec2>,
    color: Color,
    thickness: f64,
}

impl<'a> LineStringBuilder<'a> {
    fn new(commands: &'a mut RenderCommands, points: Vec<DVec2>) -> Self {
        Self {
            commands,
            points,
            thickness: 10.0,
            color: Color::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'a> Drop for LineStringBuilder<'a> {
    fn drop(&mut self) {
        for points in self.points.windows(2) {
            self.commands
                .line(points[0], points[1])
                .color(self.color)
                .thickness(self.thickness);
        }
    }
}

pub struct SpriteBuilder<'a> {
    commands: &'a mut RenderCommands,
    points: Vec<DVec2>,
    color: Color,
    thickness: f64,
}

impl<'a> SpriteBuilder<'a> {
    fn new(commands: &'a mut RenderCommands, points: Vec<DVec2>) -> Self {
        Self {
            commands,
            points,
            thickness: 10.0,
            color: Color::new(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<'a> Drop for SpriteBuilder<'a> {
    fn drop(&mut self) {
        for points in self.points.windows(2) {
            self.commands
                .line(points[0], points[1])
                .color(self.color)
                .thickness(self.thickness);
        }
    }
}
