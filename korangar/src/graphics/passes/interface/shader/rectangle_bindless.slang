[[vk::binding(0, 0)]]
cbuffer GlobalUniforms {
    float4x4 view_projection;
    float4x4 view;
    float4x4 inverse_view;
    float4x4 inverse_projection;
    float4x4 inverse_view_projection;
    float4x4 indicator_positions;
    float4 indicator_color;
    float4 ambient_color;
    float4 camera_position;
    uint2 forward_size;
    uint2 interface_size;
    uint2 pointer_position;
    float animation_timer;
    float day_timer;
    uint point_light_count;
    uint enhanced_lighting;
    uint shadow_quality;
};

struct InstanceData {
    float4 color;
    float4 corner_radius;
    float4 screen_clip;
    float2 screen_position;
    float2 screen_size;
    float2 texture_position;
    float2 texture_size;
    uint rectangle_type;
    int texture_index;
};

[[vk::binding(1, 0)]]
StructuredBuffer<InstanceData> instance_data;

[[vk::binding(2, 0)]]
SamplerState nearest_sampler;

[[vk::binding(0, 1)]]
SamplerState linear_sampler;

[[vk::binding(1, 1)]]
Texture2D textures[];

[[vk::binding(2, 1)]]
Texture2D msdf_font_map;

static const float BREATHING_ROOM = 0.5;
static const float BORDER_THRESHOLD = 2.0;
static const float EDGE_VALUE = 0.5;
static const float PXRANGE = 6.0;

struct VertexOutput {
    float4 position : SV_Position;
    float2 fragment_position : TEXCOORD0;
    float2 texture_coordinates : TEXCOORD1;
    uint instance_index : TEXCOORD2;
};

// Helper functions first
float4 vertex_data(uint vertex_index)
{
    uint index = 1u << vertex_index;
    float x = float((index & 0xEu) != 0u);
    float y = float((index & 0x1Cu) != 0u);
    return float4(x, -y, x, y);
}

float2 screen_to_clip_space(float2 screen_coords)
{
    float x = (screen_coords.x * 2.0) - 1.0;
    float y = -(screen_coords.y * 2.0) + 1.0;
    return float2(x, y);
}

float2 clip_to_screen_space(float2 ndc)
{
    float u = (ndc.x + 1.0) / 2.0;
    float v = (1.0 - ndc.y) / 2.0;
    return float2(u, v);
}

float rectangle_sdf(float2 relative_position, float2 half_size, float corner_radius)
{
    float2 shrunk_corner_position = half_size - corner_radius;
    float2 pixel_to_shrunk_corner = max(float2(0.0), abs(relative_position) - shrunk_corner_position);
    return length(pixel_to_shrunk_corner) - corner_radius + BREATHING_ROOM;
}

float median(float r, float g, float b)
{
    return max(min(r, g), min(max(r, g), b));
}

float4 calculate_msdf(float distance, float2 texture_size, float2 texture_coordinates, float4 color)
{
    float2 unit_range = (float2(PXRANGE) / texture_size);
    float2 screen_texture_size = float2(1.0) / fwidth(texture_coordinates);
    float screen_px_range = max(EDGE_VALUE * dot(unit_range, screen_texture_size), 1.0);

    return color * saturate(distance * screen_px_range + EDGE_VALUE);
}

static const float2 SAMPLE_OFFSETS[8] = {
    float2( 0.924,  0.382) * 0.5,
    float2( 0.382,  0.924) * 0.5,
    float2(-0.382,  0.924) * 0.5,
    float2(-0.924,  0.382) * 0.5,
    float2(-0.924, -0.382) * 0.5,
    float2(-0.382, -0.924) * 0.5,
    float2( 0.382, -0.924) * 0.5,
    float2( 0.924, -0.382) * 0.5
};

float4 rectangle_with_rounded_edges(float4 corner_radii, float2 screen_position, float2 screen_size,
    float2 fragment_position, float4 color)
{
    if (all(corner_radii == float4(0.0)))
    {
        return color;
    }

    float2 interface_size = float2(interface_size);
    float2 position = fragment_position * interface_size;
    float2 origin = (screen_position * interface_size) - float2(BREATHING_ROOM);
    float2 size = (screen_size * interface_size) + float2(BREATHING_ROOM * 2.0);

    float2 half_size = size * 0.5;
    float2 rectangle_center = origin + half_size;
    float2 relative_position = position - rectangle_center;

    bool is_right = relative_position.x > 0.0;
    bool is_bottom = relative_position.y > 0.0;
    float2 radii_pair = is_bottom ? corner_radii.zw : corner_radii.xy;
    float corner_radius = is_right ? radii_pair.y : radii_pair.x;

    if (corner_radius == 0.0)
    {
        return color;
    }

    float distance = rectangle_sdf(relative_position, half_size, corner_radius);
    float alpha = step(0.0, -distance);

    if (abs(distance) <= BORDER_THRESHOLD)
    {
        float total = alpha;
        for (uint index = 0; index < 8; index++)
        {
            float2 offset = SAMPLE_OFFSETS[index];
            float sample_distance = rectangle_sdf(relative_position + offset, half_size, corner_radius);
            total += step(0.0, -sample_distance);
        }
        alpha = total * (1.0 / 9.0);
    }

    return color * alpha;
}

[[shader("vertex")]]
VertexOutput vs_main(uint vertex_index : SV_VertexID, uint instance_index : SV_InstanceID)
{
    InstanceData instance = instance_data[instance_index];
    float4 vertex = vertex_data(vertex_index);

    float2 pixel_size = float2(1.0 / float(interface_size.x), 1.0 / float(interface_size.y));
    float2 size_adjustment = any(instance.corner_radius != float4(0.0)) ? (BREATHING_ROOM * 2.0) * pixel_size : float2(0.0);

    float2 adjusted_size = instance.screen_size + size_adjustment;
    float2 clip_size = adjusted_size * 2.0;
    float2 position = screen_to_clip_space(instance.screen_position) + vertex.xy * clip_size;

    VertexOutput output;
    output.position = float4(position, 0.0, 1.0);
    output.fragment_position = clip_to_screen_space(position);
    output.texture_coordinates = instance.texture_position + vertex.zw * instance.texture_size;
    output.instance_index = instance_index;
    return output;
}

[[shader("pixel")]]
float4 fs_main(VertexOutput input) : SV_Target
{
    InstanceData instance = instance_data[input.instance_index];

    float4 clip_adjustment = any(instance.corner_radius != float4(0.0)) ? float4(-0.5, -0.5, 0.5, 0.5) : float4(0.0);
    float4 adjusted_clip = instance.screen_clip + clip_adjustment;

    if (input.position.x < adjusted_clip.x || input.position.y < adjusted_clip.y ||
        input.position.x > adjusted_clip.z || input.position.y > adjusted_clip.w)
    {
        return float4(0.0, 0.0, 0.0, 0.0);
    }

    float4 color = instance.color;

    switch (instance.rectangle_type)
    {
        case 1u:
        {
            // Sprite (linear filtering)
            color *= textures[instance.texture_index].Sample(linear_sampler, input.texture_coordinates);
            break;
        }
        case 2u:
        {
            // Sprite (nearest filtering)
            color *= textures[instance.texture_index].Sample(nearest_sampler, input.texture_coordinates);
            break;
        }
        case 3u:
        {
            // SDF
            float distance = textures[instance.texture_index].Sample(linear_sampler, input.texture_coordinates).r;
            color *= float4(saturate((distance - 0.5) * 2.0 / fwidth(distance)));
            break;
        }
        case 4u:
        {
            // Text
            float4 distances = msdf_font_map.Sample(linear_sampler, input.texture_coordinates);
            float distance = median(distances.r, distances.g, distances.b) - EDGE_VALUE;
            float2 texture_size;
            msdf_font_map.GetDimensions(texture_size.x, texture_size.y);
            color = calculate_msdf(distance, texture_size, input.texture_coordinates, color);
            break;
        }
    }

    return rectangle_with_rounded_edges(
        instance.corner_radius,
        instance.screen_position,
        instance.screen_size,
        input.fragment_position,
        color
    );
}
