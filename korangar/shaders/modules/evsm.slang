#language slang 2026

module evsm;

// EVSM exponential warp constants.
// These control the tradeoff between shadow sharpness and precision.
// Higher values = sharper shadows but more precision issues.
// Recommended range: c1 = 20-40 (max 42 for FP32), c2 = 5-10
public static const var EVSM_POSITIVE_EXPONENT: float = 20.0;
public static const var EVSM_NEGATIVE_EXPONENT: float = 5.0;

// Light bleeding reduction parameter.
// Higher values reduce light bleeding but can darken shadows.
// Typical range: 0.0 (no reduction) to 0.5 (aggressive reduction)
public static const var LIGHT_BLEEDING_REDUCTION: float = 0.5;

// Minimum variance to prevent division by zero.
public static const var MIN_VARIANCE: float = 0.0001;

// VSM bias for derivative-based variance calculation.
public static const var EVSM_BIAS: float = 0.01;

/// Gets clamped EVSM exponents to prevent overflow/underflow.
/// For 32-bit format, max exponent is 42.0 to stay within fp32 range.
[ForceInline]
public func get_evsm_exponents(positive_exponent: float, negative_exponent: float) -> float2 {
    const float max_exponent = 42.0;  // Max for 32-bit format
    return min(float2(positive_exponent, negative_exponent), float2(max_exponent));
}

/// Applies exponential warp to depth value.
/// Input depth should be in [0,1], rescales to [-1,1] then applies exponentials.
/// Returns float2(pos_warp, neg_warp) where pos = exp(c1*d), neg = -exp(-c2*d)
[ForceInline]
public func warp_depth(depth: float, exponents: float2) -> float2 {
    // Rescale depth from [0,1] to [-1,1] for numerical stability.
    let rescaled = 2.0 * depth - 1.0;
    let pos = exp(exponents.x * rescaled);
    let neg = -exp(-exponents.y * rescaled);
    return float2(pos, neg);
}

[ForceInline]
public func linstep(a: float, b: float, v: float) -> float {
    return saturate((v - a) / (b - a));
}

/// Computes the 4-component exponential moments from a depth value.
[ForceInline]
public func compute_evsm_moments(depth: float) -> float4 {
    // Rescale depth from [0,1] to [-1,1] for numerical stability.
    let rescaled_depth = 2.0 * depth - 1.0;

    let pos_exp = exp(EVSM_POSITIVE_EXPONENT * rescaled_depth);
    let neg_exp = -exp(-EVSM_NEGATIVE_EXPONENT * rescaled_depth);

    // Store first and second moments: (E[warp_pos], E[warp_neg], E[warp_pos²], E[warp_neg²])
    return float4(pos_exp, neg_exp, pos_exp * pos_exp, neg_exp * neg_exp);
}

/// Applies Chebyshev's inequality to compute shadow visibility.
[ForceInline]
public func chebyshev_upper_bound(moments: float2, receiver_depth: float, min_variance: float) -> float {
    let variance = max(moments.y - (moments.x * moments.x), min_variance);

    let d = receiver_depth - moments.x;
    let p_max = variance / (variance + d * d);

    let p_reduced = reduce_light_bleeding(p_max, LIGHT_BLEEDING_REDUCTION);

    // One-tailed Chebyshev (adapted for reverse-Z)
    return (receiver_depth < moments.x) ? 1.0 : p_reduced;
}

/// Reduces light bleeding artifacts by remapping the shadow value.
/// This darkens the penumbra regions to reduce leaking.
[ForceInline]
public func reduce_light_bleeding(shadow: float, amount: float) -> float {
    return linstep(amount, 1.0, shadow);
}

/// Main EVSM shadow sampling function using 4-component moments.
/// Combines positive and negative exponentials for better quality.
/// moments.xy = (E[warp_pos], E[warp_neg]) - first moments
/// moments.zw = (E[warp_pos²], E[warp_neg²]) - second moments
[ForceInline]
public func sample_evsm_shadow(depth: float, moments: float4) -> float {
    // Get clamped exponents for 32-bit format.
    let exponents = get_evsm_exponents(EVSM_POSITIVE_EXPONENT, EVSM_NEGATIVE_EXPONENT);

    let warped_depth = warp_depth(depth, exponents);

    let depth_scale = EVSM_BIAS * 0.01 * exponents * warped_depth;
    let min_variance = depth_scale * depth_scale;

    let pos_moments = float2(moments.x, moments.z);
    let pos_contrib = chebyshev_upper_bound(pos_moments, warped_depth.x, min_variance.x);

    let neg_moments = float2(moments.y, moments.w);
    let neg_contrib = chebyshev_upper_bound(neg_moments, warped_depth.y, min_variance.y);

    return 1.0 - min(pos_contrib, neg_contrib);
}
