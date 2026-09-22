@group(0) @binding(0) var<uniform> transform: mat4x4<f32>;

struct Vertex {
    @location(0) position: vec3<f32>,
    // @location(1) color: vec4<f32>,
    // @location(2) tex_coord: vec2<f32>,
};

struct VertexShaderOut {
    @builtin(position) position: vec4<f32>,
    @location(1) height: f32,
    // @location(0) tex_coord: vec2<f32>,
    // @location(2) world_space_position: vec4<f32>,
};

@vertex
fn vs_main(vertex: Vertex, @builtin(instance_index) instanceIndex: u32) -> VertexShaderOut {
    var out: VertexShaderOut;
    out.position = transform * vec4f(vertex.position, 1.0);
    out.height = vertex.position.z;
    out.position.z = 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexShaderOut) -> @location(0) vec4<f32> {
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
    else if z > 9.0 {
        return vec4f(0.02, 0.02, 0.02, 1.0);
    }
    else if z > 8.3 {
        color = vec4f(0.1, 0.1, 0.1, 1.0);
    }
    else if z > 7.5 {
        color = vec4f(0.4, 0.2, 0.0, 1.0);
    }

    return lerp(lerp(color, vec4f(0.0, 0.0, 0.4, 1.0), 0.3), white, 0.0);
}
