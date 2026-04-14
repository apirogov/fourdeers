struct Uniforms {
    rect_origin: vec2<f32>,
    rect_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: u32,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let local = (input.pos - uniforms.rect_origin) / uniforms.rect_size;
    out.clip_pos = vec4<f32>(
        2.0 * local.x - 1.0,
        1.0 - 2.0 * local.y,
        0.0,
        1.0,
    );
    out.uv = input.uv;
    out.color = unpack4x8unorm(input.color);
    return out;
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
    let dist = abs(input.uv.y - 0.5) * 2.0;
    let aa = 1.0 - smoothstep(0.5, 1.0, dist);
    return vec4<f32>(input.color.rgb, input.color.a * aa);
}
