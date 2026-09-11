@group(0) @binding(0) var<uniform> rect_data: array<RectData, RECTS_PER_PASS>;
@group(1) @binding(0) var<uniform> height_data: array<HeightData, RECTS_PER_PASS>;

const RECTS_PER_PASS: u32 = 1300;

struct HeightData {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
    @location(1) color: vec4f,
    @location(2) uv: vec2<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) height: f32,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    let data = rect_data[vertex.instance_index];

    let dx = data.dims.x;
    let dy = data.dims.y;

    let w = data.screen_x;
    let h = data.screen_y;

    let a = data.pos;
    let b = data.pos + rotate_vector(vec2<f32>(dx,  0.0), data.angle);
    let c = data.pos + rotate_vector(vec2<f32>(dx,  dy),  data.angle);
    let d = data.pos + rotate_vector(vec2<f32>(0.0, dy),  data.angle);

    let dims = vec2<f32>(w, h);

    let positions = array<vec2<f32>, 4>(
        a / dims,
        b / dims,
        c / dims,
        d / dims,
    );

    let height = height_data[vertex.instance_index];

    let heights = array<f32, 4>(
        height.a,
        height.b,
        height.c,
        height.d,
    );

    var pos = positions[vertex.vertex_index] * 2.0 - 1.0;
    out.position = vec4<f32>(pos, 1.0, 1.0);

    let z = heights[vertex.vertex_index];

    out.height = z;
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    let levels = 20.0;
    let z = discretize(in.height, levels);

    let g = discretize(smoothstep(0.0, 16.0, z) * 0.4 + 0.3, levels);

    var color = vec4f(0.0, g, 0.0, 1.0);

    let white = vec4f(1.0, 1.0, 1.0, 1.0);

    if z < -2.0 {
        color = vec4f(0.2, 0.1, 0.5, 1.0);
    }
    else if z < 0.0 {
        color = vec4f(0.2, 0.2, 1.0, 1.0);
    }
    else if z > 25.0 {
        return vec4f(0.1, 0.1, 0.1, 1.0);
    }
    else if z > 22.0 {
        color = vec4f(0.1, 0.1, 0.0, 1.0);
    }
    else if z > 19.0 {
        color = vec4f(0.6, 0.3, 0.0, 1.0);
    }

    return lerp(lerp(color, vec4f(0.0, 0.0, 0.4, 1.0), 0.3), white, 0.2);
}
