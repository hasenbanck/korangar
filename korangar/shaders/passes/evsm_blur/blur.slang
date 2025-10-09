#language slang 2026

import screen_space;

// 9-tap Gaussian blur optimized to 5 texture fetches using linear sampling
// Based on: https://www.rastergrid.com/blog/2010/09/efficient-gaussian-blur-with-linear-sampling/
//
// Original 9-tap Gaussian weights (sigma = 2.0, normalized):
// Index:   -4     -3     -2     -1      0     +1     +2     +3     +4
// Weight: 0.028  0.051  0.095  0.122  0.137  0.122  0.095  0.051  0.028
//
// Optimization: Combine adjacent samples using hardware bilinear filtering
// This reduces 9 texture fetches to 5 fetches (~44% reduction)

// Optimized 5-tap samples with calculated offsets for linear interpolation:
// Offset calculation: (w1 * offset1 + w2 * offset2) / (w1 + w2)
static const var OFFSETS: float[4] = {
    1.4118,   // Combines offsets -1, -2: (0.122*1 + 0.095*2) / (0.122+0.095) = 1.4118
    3.2941,   // Combines offsets -3, -4: (0.051*3 + 0.028*4) / (0.051+0.028) = 3.2941
    1.4118,   // Combines offsets +1, +2 (symmetric)
    3.2941    // Combines offsets +3, +4 (symmetric)
};

static const var WEIGHTS: float[5] = {
    0.137,    // Center (index 0)
    0.217,    // Sum of weights at -1, -2: 0.122 + 0.095 = 0.217
    0.079,    // Sum of weights at -3, -4: 0.051 + 0.028 = 0.079
    0.217,    // Sum of weights at +1, +2 (symmetric)
    0.079     // Sum of weights at +3, +4 (symmetric)
};

[[vk::constant_id(0)]] const var IS_HORIZONTAL: bool;

[[vk::binding(2, 0)]] var linear_sampler: SamplerState;
[[vk::binding(0, 1)]] var input_texture: Texture2D;

[[shader("vertex")]]
func vs_main(uint vertex_index : SV_VulkanVertexID) -> FullscreenVertex {
    return FullscreenVertex.new(vertex_index);
}

[[shader("fragment")]]
func fs_main(input: FullscreenVertex) -> float4 {
    uint width, height;
    input_texture.GetDimensions(width, height);
    let texel_size = float2(1.0 / float(width), 1.0 / float(height));

    var result = float4(0.0);

    // Center sample (no offset)
    result += input_texture.Sample(linear_sampler, input.uv) * WEIGHTS[0];

    [unroll]
    for (var i = 0; i < 4; i++) {
        let sign = (i < 2) ? -1.0 : 1.0;  // Negative offsets for first two, positive for last two
        let offset = IS_HORIZONTAL
            ? float2(sign * OFFSETS[i] * texel_size.x, 0.0)
            : float2(0.0, sign * OFFSETS[i] * texel_size.y);
        result += input_texture.Sample(linear_sampler, input.uv + offset) * WEIGHTS[i + 1];
    }

    return result;
}
