@group(0) @binding(0) var wall_atlas: texture_2d_array<f32>;
@group(0) @binding(1) var wall_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2f,
    @location(1) uv: vec2f,
    @location(2) screen_x: f32,
    @location(3) top: f32,
    @location(4) height: f32,
    @location(5) tex_u: f32,
    @location(6) tex_layer: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2f,
    @location(1) tex_layer: u32,
};

const SCREEN_W = 854;
const SCREEN_H = 480;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let pixel_x = input.screen_x + input.position.x;
    let pixel_y = input.top + input.position.y * input.height;
    let ndc_x = (pixel_x / SCREEN_W) * 2.0 - 1.0;
    let ndc_y = 1.0 - (pixel_y / SCREEN_H) * 2.0;

    out.clip_position = vec4f(ndc_x, ndc_y, 0.0, 1.0);
    out.tex_coords = vec2f(input.tex_u, input.position.y);
    out.tex_layer = input.tex_layer;

    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(wall_atlas, wall_sampler, input.tex_coords, input.tex_layer);
    // return vec4f(1.0, 0.0, 0.0, 1.0);
    return vec4f(color.rgb, 1.0);
}
