use crate::*;
use glam::DVec2;
use glm::*;
use noise::{NoiseFn, Perlin};
use wgpu::util::DeviceExt;

pub struct Mesh {
    buffer: wgpu::Buffer,
    offset: u64,
    index_count: u32,
}

impl Mesh {
    pub fn index_count(&self) -> u32 {
        self.index_count
    }

    pub fn vertex_buffer(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(0..self.offset)
    }

    pub fn index_buffer(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(self.offset..)
    }

    pub fn index_format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint16
    }

    pub fn set_as_active(&self, rp: &mut wgpu::RenderPass) {
        rp.set_vertex_buffer(0, self.vertex_buffer());
        rp.set_index_buffer(self.index_buffer(), self.index_format());
    }
}

fn vertices_to_bytes<T: Vertex>(vertices: &[T]) -> Vec<u8> {
    vertices
        .iter()
        .map(|v| v.to_bytes().to_vec())
        .collect::<Vec<Vec<u8>>>()
        .concat()
}

fn indices_to_bytes(indices: &[u16]) -> Vec<u8> {
    indices
        .iter()
        .map(|i| i.to_le_bytes().to_vec())
        .collect::<Vec<Vec<u8>>>()
        .concat()
}

pub fn mesh_from_vi<T: Vertex>(device: &wgpu::Device, vertices: &[T], indices: &[u16]) -> Mesh {
    let bytes_1: &[u8] = &vertices_to_bytes(vertices);
    let bytes_2: &[u8] = &indices_to_bytes(&indices);
    let bytes_merged: &[u8] = &[bytes_1, bytes_2].concat();

    let buffer_descriptor = wgpu::util::BufferInitDescriptor {
        label: Some("Quad vertex & index buffer"),
        contents: bytes_merged,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX,
    };

    let buffer = device.create_buffer_init(&buffer_descriptor);
    let offset: u64 = bytes_1.len().try_into().unwrap();

    Mesh {
        buffer,
        offset,
        index_count: indices.len() as u32,
    }
}

pub fn make_quad(device: &wgpu::Device) -> Mesh {
    let size = 1.0;
    let vertices: [FullVertex; 4] = [
        FullVertex::new(
            Vec3::new(-size, -size, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
            Vec2::new(0.0, 0.0),
        ),
        FullVertex::new(
            Vec3::new(size, -size, 0.0),
            Vec4::new(0.0, 1.0, 1.0, 1.0),
            Vec2::new(1.0, 0.0),
        ),
        FullVertex::new(
            Vec3::new(size, size, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
            Vec2::new(1.0, 1.0),
        ),
        FullVertex::new(
            Vec3::new(-size, size, 0.0),
            Vec4::new(1.0, 0.0, 1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ),
    ];

    let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

    mesh_from_vi(device, &vertices, &indices)
}

pub fn make_quad_01(device: &wgpu::Device) -> Mesh {
    let vertices: [FullVertex; 4] = [
        FullVertex::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
            Vec2::new(0.0, 0.0),
        ),
        FullVertex::new(
            Vec3::new(1.0, 0.0, 0.0),
            Vec4::new(0.0, 1.0, 1.0, 1.0),
            Vec2::new(1.0, 0.0),
        ),
        FullVertex::new(
            Vec3::new(1.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
            Vec2::new(1.0, 1.0),
        ),
        FullVertex::new(
            Vec3::new(0.0, 1.0, 0.0),
            Vec4::new(1.0, 0.0, 1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ),
    ];

    let indices: [u16; 6] = [0, 1, 2, 2, 3, 0];

    mesh_from_vi(device, &vertices, &indices)
}

pub fn make_n_gon(device: &wgpu::Device, n: usize, color: Color) -> Mesh {
    let vertices = (0..n)
        .map(|i| {
            let a = 2.0 * std::f32::consts::PI * i as f32 / n as f32;
            let x = a.cos();
            let y = a.sin();
            (a, x, y)
        })
        .map(|(a, x, y)| {
            let pos = Vec3::new(x, y, 0.0);
            let r = a.sin() * 0.5 + 0.5;
            let g = a.cos() * 0.5 + 0.5;
            let b = (a * 0.5).sin() * 0.5 + 0.5;
            let color = color.to_vec();
            let tx = Vec2::new(x * 0.5 + 0.5, y * 0.5 + 0.5);
            FullVertex::new(pos, color, tx)
        })
        .collect::<Vec<_>>();

    let indices: Vec<u16> = (0..n - 2)
        .map(|i| {
            let i = i as u16;
            [0, 1 + i, 2 + i]
        })
        .collect::<Vec<_>>()
        .concat();

    mesh_from_vi(device, &vertices, &indices)
}

pub fn quad_indices_to_tris(a: u16, b: u16, c: u16, d: u16) -> [u16; 6] {
    [a, b, c, a, c, d]
}

pub fn make_cube(device: &wgpu::Device, color: Vec4) -> Mesh {
    let x = 0.5;
    let y = 0.5;
    let z = 0.5;
    let vertices = vec![
        FullVertex::new(Vec3::new(-x, -y, -z), color, Vec2::new(0.0, 0.0)),
        FullVertex::new(Vec3::new(x, -y, -z), color, Vec2::new(1.0, 0.0)),
        FullVertex::new(Vec3::new(x, y, -z), color, Vec2::new(1.0, 1.0)),
        FullVertex::new(Vec3::new(-x, y, -z), color, Vec2::new(0.0, 1.0)),
        FullVertex::new(Vec3::new(-x, -y, z), color, Vec2::new(0.0, 0.0)),
        FullVertex::new(Vec3::new(x, -y, z), color, Vec2::new(1.0, 0.0)),
        FullVertex::new(Vec3::new(x, y, z), color, Vec2::new(1.0, 1.0)),
        FullVertex::new(Vec3::new(-x, y, z), color, Vec2::new(0.0, 1.0)),
    ];

    let indices = [
        quad_indices_to_tris(3, 2, 1, 0),
        quad_indices_to_tris(4, 5, 6, 7),
        quad_indices_to_tris(0, 1, 5, 4),
        quad_indices_to_tris(2, 3, 7, 6),
        quad_indices_to_tris(3, 0, 4, 7),
        quad_indices_to_tris(1, 2, 6, 5),
    ]
    .concat();

    mesh_from_vi(device, &vertices, &indices)
}

pub fn make_tetrahedron(device: &wgpu::Device) -> Mesh {
    let inv_rt2 = 1.0 / 2.0f32.sqrt();

    let vertices = vec![
        FullVertex::new(
            Vec3::new(1.0, -inv_rt2, 0.0),
            Vec4::new(1.0, 0.0, 0.0, 1.0),
            Vec2::new(0.0, 0.0),
        ),
        FullVertex::new(
            Vec3::new(-1.0, -inv_rt2, 0.0),
            Vec4::new(0.0, 1.0, 0.0, 1.0),
            Vec2::new(0.0, 0.0),
        ),
        FullVertex::new(
            Vec3::new(0.0, inv_rt2, 1.0),
            Vec4::new(0.0, 0.0, 1.0, 1.0),
            Vec2::new(0.0, 0.0),
        ),
        FullVertex::new(
            Vec3::new(0.0, inv_rt2, -1.0),
            Vec4::new(1.0, 0.0, 1.0, 1.0),
            Vec2::new(0.0, 0.0),
        ),
    ];

    #[rustfmt::skip]
    let indices = vec![
        2, 1, 0,
        1, 2, 3,
        0, 1, 3,
        3, 2, 0,
    ];

    mesh_from_vi(device, &vertices, &indices)
}

pub fn draw_mesh(rp: &mut wgpu::RenderPass, mesh: &Mesh) {
    mesh.set_as_active(rp);
    rp.draw_indexed(0..mesh.index_count(), 0, 0..1);
}
