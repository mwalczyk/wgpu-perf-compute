use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    position: [f32; 4],
    uv: [f32; 2],
}

impl Vertex {
    pub fn new(position: [f32; 4], uv: [f32; 2]) -> Self {
        Self { position, uv }
    }
}