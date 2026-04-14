pub(crate) mod callback;
pub(crate) mod pipeline;
pub(crate) mod vertex;

use eframe::{egui, egui_wgpu, wgpu};
use egui_wgpu::RenderState;

use self::callback::GpuCallback;
use self::pipeline::GpuPipeline;

pub(crate) struct GpuRenderer {
    target_format: wgpu::TextureFormat,
}

impl GpuRenderer {
    pub fn new(rs: &RenderState) -> Self {
        let target_format = rs.target_format;

        let pipeline = GpuPipeline::new(&rs.device, target_format);
        rs.renderer.write().callback_resources.insert(pipeline);

        Self { target_format }
    }

    pub fn submit(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        vertices: Vec<vertex::GpuVertex>,
        indices: Vec<u32>,
        screen_size: [f32; 2],
    ) {
        if indices.is_empty() {
            return;
        }

        let callback = GpuCallback::new(vertices, indices, screen_size);
        let cb = egui_wgpu::Callback::new_paint_callback(rect, callback);
        painter.add(egui::Shape::Callback(cb));
    }

    pub fn target_format(&self) -> wgpu::TextureFormat {
        self.target_format
    }
}
