struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_coordinate: vec2<f32>,
}

@group(2) @binding(0) var source_texture: texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let uv = vec2<f32>(f32((vertex_index << 1u) & 2u), f32(vertex_index & 2u));
    var output: VertexOutput;
    output.position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
    output.texture_coordinate = uv;
    return output;
}

@fragment
fn fs_horizontal(@location(0) texture_coordinate: vec2<f32>) -> @location(0) vec4<f32> {
    let texture_dimensions = textureDimensions(source_texture);
    let pixel_coords = vec2<i32>(texture_coordinate * vec2<f32>(texture_dimensions));

    var color = vec4<f32>(0.0);
    var kernel_size = 0;

    switch(texture_dimensions.x) {
        case 8192u: { kernel_size = 7; }
        case 4096u: { kernel_size = 5; }
        case 2048u: { kernel_size = 3; }
        default: { kernel_size = 1; }
    }

    let kernel_offset = kernel_size / 2;
    let weight = 1.0 / f32(kernel_size);

    for(var i = 0; i < kernel_size; i++) {
        let offset = i - kernel_offset;
        let sample_position = vec2<i32>(pixel_coords.x + offset, pixel_coords.y);
        let clamped_position = clamp(sample_position, vec2<i32>(0), vec2<i32>(texture_dimensions) - 1);
        color += textureLoad(source_texture, clamped_position, 0) * weight;
    }

    return vec4<f32>(color.rgb, 1.0);
}

@fragment
fn fs_vertical(@location(0) texture_coordinate: vec2<f32>) -> @location(0) vec4<f32> {
    let texture_dimensions = textureDimensions(source_texture);
    let pixel_coords = vec2<i32>(texture_coordinate * vec2<f32>(texture_dimensions));

    var color = vec4<f32>(0.0);
    var kernel_size = 0;

    switch(texture_dimensions.x) {
        case 8192u: { kernel_size = 7; }
        case 4096u: { kernel_size = 5; }
        case 2048u: { kernel_size = 3; }
        default: { kernel_size = 1; }
    }

    let kernel_offset = kernel_size / 2;
    let weight = 1.0 / f32(kernel_size);

    for(var i = 0; i < kernel_size; i++) {
        let offset = i - kernel_offset;
        let sample_position = vec2<i32>(pixel_coords.x, pixel_coords.y + offset);
        let clamped_position = clamp(sample_position, vec2<i32>(0), vec2<i32>(texture_dimensions) - 1);
        color += textureLoad(source_texture, clamped_position, 0) * weight;
    }

    return vec4<f32>(color.rgb, 1.0);
}
