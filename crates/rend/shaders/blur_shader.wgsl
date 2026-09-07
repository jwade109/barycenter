@group(0) @binding(0) var texture: texture_2d<f32>;
@group(0) @binding(1) var sample: sampler;
@group(1) @binding(0) var<uniform> blur_params: BlurParams;

struct BlurParams
{
    resolution: vec2f,
    kernel_size: f32,
    is_vertical: i32,
    is_nullopt: i32,
};

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexShaderOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(vertex: Vertex) -> VertexShaderOutput {
    var out: VertexShaderOutput;
    out.position = vec4<f32>(vertex.position, 1.0);
    out.uv = vertex.uv;
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

fn gaussian_weights(n: u32) -> array<f32, 3> {
    return array<f32, 3>(0.0044, 0.0540, 0.2420);
}

fn do_blur(in: VertexShaderOutput) -> vec4<f32> {

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    // const weights = array<f32, 10>(0.0044, 0.0540, 0.2420, 0.3991, 0.2420, 0.0540, 0.0044, 0.0, 0.0, 0.0);

    let off = vec2f(1.0, 1.0) / blur_params.resolution;

    let n = max(min(u32(round(blur_params.kernel_size)), 1000u), 1u);

    // let dddd = gaussian_weights(n);

    if blur_params.is_vertical > 0 {
        for (var i = 0u; i < n; i = i + 1u) {
            let w = 1.0 / f32(n);
            let y = f32(i) - f32(n) / 2.0;
            let uv = in.uv + vec2<f32>(0.0, off.y * y);
            color += textureSample(texture, sample, uv) * w;
        }
    } else {
        for (var i = 0u; i < n; i = i + 1u) {
            let w = 1.0 / f32(n);
            let x = f32(i) - f32(n) / 2.0;
            let uv = in.uv + vec2<f32>(off.x * x, 0.0);
            color += textureSample(texture, sample, uv) * w;
        }
    }

    color.w = 1.0;

    return color;
}

@fragment
fn fs_main(in: VertexShaderOutput) -> @location(0) vec4<f32> {
    if blur_params.is_nullopt > 0 {
        return textureSample(texture, sample, in.uv);
    }
    return do_blur(in);
}
