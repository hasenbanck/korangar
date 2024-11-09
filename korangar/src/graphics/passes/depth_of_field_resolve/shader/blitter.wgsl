@group(0) @binding(0) var color_texture: texture_2d<f32>;
@group(1) @binding(0) var depth_texture: texture_depth_2d;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
    @location(1) depth: f32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Full screen triangle.
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let output = FragmentOutput();
    output.color = textureLoad(color_texture, vec2<i32>(position.xy), 0);
    output.depth = textureLoad(depth_texture, vec2<i32>(position.xy), 0);
    return output;
}
