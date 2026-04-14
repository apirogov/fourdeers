pub(crate) mod callback;
pub(crate) mod pipeline;
pub(crate) mod vertex;

pub(crate) use vertex::GpuVertex;

use eframe::{egui, egui_wgpu};
use egui_wgpu::RenderState;

use self::callback::GpuCallback;
use self::pipeline::GpuPipeline;

pub(crate) struct GpuRenderer;

impl GpuRenderer {
    pub fn try_new(rs: &RenderState) -> Option<Self> {
        let target_format = rs.target_format;

        let pipeline = GpuPipeline::try_new(&rs.device, target_format)?;
        rs.renderer.write().callback_resources.insert(pipeline);

        Some(Self)
    }

    pub fn submit(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        vertices: Vec<vertex::GpuVertex>,
        indices: Vec<u32>,
    ) {
        if indices.is_empty() {
            return;
        }

        let rect_origin = [rect.min.x, rect.min.y];
        let rect_size = [rect.width(), rect.height()];
        let callback = GpuCallback::new(vertices, indices, rect_origin, rect_size);
        let cb = egui_wgpu::Callback::new_paint_callback(rect, callback);
        painter.add(egui::Shape::Callback(cb));
    }
}
