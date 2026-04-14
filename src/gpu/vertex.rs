use eframe::{egui, wgpu};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GpuVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: u32,
}

impl GpuVertex {
    const WHITE_UV: egui::Pos2 = egui::Pos2::new(0.0, 0.0);

    pub fn new(pos: egui::Pos2, color: egui::Color32) -> Self {
        Self {
            pos: [pos.x, pos.y],
            uv: [Self::WHITE_UV.x, Self::WHITE_UV.y],
            color: bytemuck::cast(color),
        }
    }

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
    0 => Float32x2,
    1 => Float32x2,
    2 => Uint32,
];
