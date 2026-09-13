@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var sample: sampler;
@group(1) @binding(0) var<uniform> rect_data: array<RectData, RECTS_PER_PASS>;

const RECTS_PER_PASS: u32 = 1300;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) instance_index: u32,
    @location(1) uv: vec2f,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;

    let data = rect_data[vertex.instance_index];

    let w = data.screen_x;
    let h = data.screen_y;

    let corners = rect_corners(data.pos, data.dims, data.angle);

    let dims = vec2<f32>(w, h);

    let uvs = array<vec2<f32>, 4>(
        vec2f(0.0, 1.0),
        vec2f(1.0, 1.0),
        vec2f(1.0, 0.0),
        vec2f(0.0, 0.0),
    );

    var v = corners[vertex.vertex_index] / dims * 2.0 - 1.0;
    out.position = vec4<f32>(v, 1.0, 1.0);
    out.uv = uvs[vertex.vertex_index];
    out.instance_index = vertex.instance_index;
    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    let data = rect_data[in.instance_index];
    let tint_color = vec4f(data.r, data.g, data.b, data.a);
    var col = textureSample(texture, sample, in.uv);
    col = vec4f(col.xyz, col.w);
    col = col * tint_color;
    return col;
}
