@group(0) @binding(0) var color_texture: texture_multisampled_2d<f32>;
@group(1) @binding(0) var depth_texture: texture_depth_multisampled_2d;

override SAMPLE_COUNT: i32;

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
fn fs_main(@builtin(position) position: vec4<f32>) -> FragmentOutput {
    let output = FragmentOutput();

    var color = vec4<f32>(0.0);
    var depth = f32(0.0);

    for (var sample_id: i32 = 0; sample_id < SAMPLE_COUNT; sample_id++) {
        color += textureLoad(color_texture, pixel_coord, sample_id);
        depth += textureLoad(depth_texture, pixel_coord, sample_id);
    }

    output.color = color / f32(SAMPLE_COUNT);
    output.depth = depth / f32(SAMPLE_COUNT);
    return output;
}
