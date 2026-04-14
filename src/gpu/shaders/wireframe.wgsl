struct Uniforms {
    screen_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: u32,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = vec4<f32>(
        2.0 * input.pos.x / uniforms.screen_size.x - 1.0,
        1.0 - 2.0 * input.pos.y / uniforms.screen_size.y,
        0.0,
        1.0,
    );
    out.color = unpack4x8unorm(input.color);
    return out;
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
