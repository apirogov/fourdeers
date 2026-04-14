use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use eframe::{egui, egui_wgpu, wgpu};
use egui::PaintCallbackInfo;
use egui_wgpu::{CallbackResources, CallbackTrait, ScreenDescriptor};
use wgpu::util::DeviceExt;

use super::pipeline::GpuPipeline;
use super::vertex::GpuVertex;

static NEXT_CALLBACK_ID: AtomicU64 = AtomicU64::new(0);

pub(crate) struct GpuFrameMap(HashMap<u64, GpuFrameResources>);

impl GpuFrameMap {
    fn new() -> Self {
        Self(HashMap::new())
    }
}

struct GpuFrameResources {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    index_count: u32,
}

pub(crate) struct GpuCallback {
    id: u64,
    vertices: Vec<GpuVertex>,
    indices: Vec<u32>,
    screen_size: [f32; 2],
}

impl GpuCallback {
    pub fn new(vertices: Vec<GpuVertex>, indices: Vec<u32>, screen_size: [f32; 2]) -> Self {
        Self {
            id: NEXT_CALLBACK_ID.fetch_add(1, Ordering::Relaxed),
            vertices,
            indices,
            screen_size,
        }
    }
}

impl CallbackTrait for GpuCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if self.vertices.is_empty() || self.indices.is_empty() {
            return Vec::new();
        }

        let pipeline = callback_resources
            .get::<GpuPipeline>()
            .expect("GpuPipeline not found");

        queue.write_buffer(
            &pipeline.uniform_buffer,
            0,
            bytemuck::cast_slice(&[super::pipeline::Uniforms {
                screen_size: self.screen_size,
            }]),
        );

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fourdeers_vertices"),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fourdeers_indices"),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let bind_group = pipeline.create_bind_group(device);

        let frame_map = callback_resources
            .entry::<GpuFrameMap>()
            .or_insert_with(GpuFrameMap::new);

        frame_map.0.insert(
            self.id,
            GpuFrameResources {
                vertex_buffer,
                index_buffer,
                bind_group,
                index_count: self.indices.len() as u32,
            },
        );

        Vec::new()
    }

    fn paint(
        &self,
        info: PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &CallbackResources,
    ) {
        let pipeline = callback_resources
            .get::<GpuPipeline>()
            .expect("GpuPipeline not found");

        let frame_map = match callback_resources.get::<GpuFrameMap>() {
            Some(m) => m,
            None => return,
        };

        let frame = match frame_map.0.get(&self.id) {
            Some(f) => f,
            None => return,
        };

        if frame.index_count == 0 {
            return;
        }

        let viewport = info.viewport_in_pixels();
        render_pass.set_viewport(
            viewport.left_px as f32,
            viewport.from_bottom_px as f32,
            viewport.width_px as f32,
            viewport.height_px as f32,
            0.0,
            1.0,
        );

        render_pass.set_pipeline(&pipeline.pipeline);
        render_pass.set_bind_group(0, &frame.bind_group, &[]);
        render_pass.set_vertex_buffer(0, frame.vertex_buffer.slice(..));
        render_pass.set_index_buffer(frame.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..frame.index_count, 0, 0..1);
    }
}
