#version 330 core

in vec2 vTexCoord;
out vec4 FragColor;

uniform sampler2D uSceneTexture;
uniform sampler2D uBloomTexture;
layout (std140) uniform GlobalData {
    vec2 uSize;
    float uTexRatio;
    float uBloomIntensity;
};
uniform int uToneMappingMode;

// 0 = Reinhard
// 1 = Reinhard Extended
// 2 = ACES
// 3 = Uncharted 2
// 4 = AgX Tone Mapping
// 5 = Khronos PBR Neutral

// --- 1. Reinhard Tone Mapping ---
vec3 reinhard(vec3 color) {
    return color / (color + vec3(1.0));
}

// --- 2. Reinhard Extended Tone Mapping ---
// Allows high luminance to burn out to white
vec3 reinhardExtended(vec3 color) {
    float whitePoint = 4.0; // Max luminance that maps to 1.0
    vec3 numerator = color * (vec3(1.0) + (color / (whitePoint * whitePoint)));
    return numerator / (vec3(1.0) + color);
}

// --- 3. ACES Filmic Tone Mapping ---
// Narkowicz approximation
vec3 aces(vec3 color) {
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), 0.0, 1.0);
}

// --- 4. Uncharted 2 (Hable) Tone Mapping ---
vec3 uncharted2Tonemap(vec3 x) {
    float A = 0.15;
    float B = 0.50;
    float C = 0.10;
    float D = 0.20;
    float E = 0.02;
    float F = 0.30;
    return ((x * (A * x + C * B) + D * E) / (x * (A * x + B) + D * F)) - E / F;
}

vec3 uncharted2(vec3 color) {
    float exposure_bias = 2.0;
    vec3 curr = uncharted2Tonemap(exposure_bias * color);
    vec3 whiteScale = 1.0 / uncharted2Tonemap(vec3(11.2));
    return curr * whiteScale;
}

// --- 5. Khronos PBR Neutral Tone Mapping ---
vec3 khronosPBR(vec3 color) {
    const float startCompression = 0.8 - 0.04;
    const float desaturation = 0.15;

    float x = min(color.r, min(color.g, color.b));
    float offset = x < 0.08 ? x - 6.25 * x * x : 0.04;
    color -= offset;

    float peak = max(color.r, max(color.g, color.b));
    if (peak < startCompression) return color;

    const float d = 1.0 - startCompression;
    float newPeak = 1.0 - d * d / (peak + d - startCompression);
    color *= newPeak / peak;

    float g = 1.0 - 1.0 / (desaturation * (peak - newPeak) + 1.0);
    return mix(color, newPeak * vec3(1, 1, 1), g);
}

// --- 6. AgX Tone Mapping ---
// AgX
// https://github.com/sobotka/AgX
// https://www.shadertoy.com/view/cd3XWr

// Mean error^2: 3.6705141e-06
vec3 agxDefaultContrastApprox(vec3 x) {
    vec3 x2 = x * x;
    vec3 x4 = x2 * x2;

    return + 15.5     * x4 * x2
           - 40.14    * x4 * x
           + 31.96    * x4
           - 6.868    * x2 * x
           + 0.4298   * x2
           + 0.1191   * x
           - 0.00232;
}

vec3 agx(vec3 val) {
    const mat3 agx_mat = mat3(
        0.842479062253094, 0.0423282422610123, 0.0423756549057051,
        0.0784335999999992,  0.878468636469772,  0.0784336,
        0.0792237451477643, 0.0791661274605434, 0.879142973793104);

    const float min_ev = -12.47393f;
    const float max_ev = 4.026069f;

    // Input transform
    val = agx_mat * val;

    // Log2 space encoding
    val = clamp(log2(val), min_ev, max_ev);
    val = (val - min_ev) / (max_ev - min_ev);

    // Apply sigmoid function approximation
    val = agxDefaultContrastApprox(val);

    return val;
}

vec3 agxEotf(vec3 val) {
    const mat3 agx_mat_inv = mat3(
        1.19687900512017, -0.0528968517574562, -0.0529716355144438,
        -0.0980208811401368, 1.15190312990417, -0.0980434501171241,
        -0.0990297440797205, -0.0989611768448433, 1.15107367264116);

    // Undo input transform
    val = agx_mat_inv * val;

    // I enabled this line to do linear to srgb in line 180 for all tonemappings.
    // sRGB IEC 61966-2-1 2.2 Exponent Reference EOTF Display
    // val = pow(val, vec3(2.2));

    return val;
}

vec3 agxLook(vec3 val) {
    const vec3 lw = vec3(0.2126, 0.7152, 0.0722);
    float luma = dot(val, lw);

    // Default look
    vec3 offset = vec3(0.0);
    vec3 slope = vec3(1.0);
    vec3 power = vec3(1.0, 1.0, 1.0);
    float sat = 1.22;

    // ASC CDL
    val = pow(val * slope + offset, power);
    return luma + sat * (val - luma);
}

vec3 tonemapping_AgX(vec3 color)
{
    color = agx(color);
    color = agxLook(color);
    color = agxEotf(color);
    return color;
}

void main() {
    vec3 sceneColor = texture(uSceneTexture, vTexCoord).rgb;
    vec3 bloomColor = texture(uBloomTexture, vTexCoord).rgb;

    // Additive blending with intensity control
    vec3 result = sceneColor + bloomColor * uBloomIntensity;

    // Apply tone mapping
    if (uToneMappingMode == 0) {
        result = reinhard(result);
    } else if (uToneMappingMode == 1) {
        result = reinhardExtended(result);
    } else if (uToneMappingMode == 2) {
        result = aces(result);
    } else if (uToneMappingMode == 3) {
        result = uncharted2(result);
    } else if (uToneMappingMode == 4) {
        result = tonemapping_AgX(result);
    } else if (uToneMappingMode == 5) {
        result = khronosPBR(result);
    } else {
        // Fallback to Khronos PBR
        result = khronosPBR(result);
    }

    // Gamma correction
    result = pow(result, vec3(1.0 / 2.2));

    FragColor = vec4(result, 1.0);
}
