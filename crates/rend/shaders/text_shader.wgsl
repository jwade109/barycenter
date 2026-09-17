@group(0) @binding(0) var the_texture: texture_2d<f32>;
@group(0) @binding(1) var the_sampler: sampler;
@group(1) @binding(0) var<uniform> color_array: array<vec4<f32>, MAX_CHARS_PER_PASS>;
@group(2) @binding(0) var<uniform> sample_info_array: array<SampleInfo, MAX_CHARS_PER_PASS>;
@group(3) @binding(0) var<uniform> transforms_array: array<TextTransform, MAX_CHARS_PER_PASS>;

const MAX_CHARS_PER_PASS: u32 = 1400;

struct SampleInfo {
    origin_x: u32,
    origin_y: u32,
    sample_width: u32,
    sample_height: u32,
    image_width: u32,
    image_height: u32,
    _pad1: u32,
    _pad2: u32,
};

struct TextTransform {
    x:      f32,
    y:      f32,
    width:  f32,
    height: f32,
    angle:  f32,
    sx:     f32,
    sy:     f32,
    z:      f32,
}

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

    out.instance_index = vertex.instance_index;

    let data = transforms_array[vertex.instance_index];
    let sample = sample_info_array[vertex.instance_index];

    let root = vec2f(data.x, data.y);
    let dx = data.width;
    let dy = data.height;

    let w = data.sx;
    let h = data.sy;

    let a = root;
    let b = root + rotate_vector(vec2<f32>(dx,  0.0), data.angle);
    let c = root + rotate_vector(vec2<f32>(dx,  dy),  data.angle);
    let d = root + rotate_vector(vec2<f32>(0.0, dy),  data.angle);

    let dims = vec2<f32>(w, h);

    let positions = array<vec2<f32>, 4>(
        a / dims,
        b / dims,
        c / dims,
        d / dims,
    );

    let uvs = array<vec2<f32>, 4>(
        vec2f(0.0, 1.0),
        vec2f(1.0, 1.0),
        vec2f(1.0, 0.0),
        vec2f(0.0, 0.0),
    );

    var uv = uvs[vertex.vertex_index];

    uv.x = (f32(sample.origin_x) + uv.x * f32(sample.sample_width)) / f32(sample.image_width);
    uv.y = (f32(sample.origin_y) + uv.y * f32(sample.sample_height)) / f32(sample.image_height);

    var pos = positions[vertex.vertex_index] * 2.0 - 1.0;
    out.position = vec4<f32>(pos, data.z, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    // return vec4f(in.uv, 0.0, 1.0);
    var c = textureSample(the_texture, the_sampler, in.uv);
    c.x = pow(c.x, 2.0);
    c.y = pow(c.y, 2.0);
    c.z = pow(c.z, 2.0);

    let l = length(c.xyz);

    let col = color_array[in.instance_index];

    // for debugging
    // if l < 0.03 {
    //     return vec4<f32>(col.xyz, 0.3);
    // }

    // let alpha = sqrt(sqrt(round(l * 5.0) / 5.0));
    // let alpha = length(c.xyz);
    let alpha = smoothstep(0.23, 0.28, l);

    return vec4<f32>(col.xyz * alpha, col.w * alpha);
}