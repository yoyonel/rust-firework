#version 330 core

in vec2 vTexCoord;
out vec4 FragColor;

uniform sampler2D uSceneTexture;
uniform sampler2D uBloomTexture;
uniform float uBloomIntensity;
uniform int uToneMappingMode;

// 0 = Reinhard
// 1 = Reinhard Extended
// 2 = ACES
// 3 = Uncharted 2
// 4 = AgX
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

// --- 5. AgX Tone Mapping (Approximation) ---
// Based on standard AgX implementation for real-time
vec3 agx(vec3 color) {
    // AgX Input Transform
    const mat3 agx_input_mat = mat3(
        0.842479062253094, 0.0423282422610123, 0.0423756549057051,
        0.0784335999999992, 0.878468636469772, 0.0784336,
        0.0792237458965401, 0.0791661274605434, 0.879142973793104
    );
    
    vec3 val = agx_input_mat * color;
    
    // Log2 space encoding
    const float min_ev = -12.47393;
    const float max_ev = 4.026069;
    val = clamp(log2(val), min_ev, max_ev);
    val = (val - min_ev) / (max_ev - min_ev);
    
    // Sigmoid function (Apply curve)
    // Simple sigmoid approximation for AgX curve
    val = val * val * (3.0 - 2.0 * val); // Hermite interpolation as a simple S-curve
    
    // AgX Output Transform (Inverse of Input to some extent, but mapping to sRGB)
    // This is a simplified approximation of the "AgX Look"
    const mat3 agx_output_mat = mat3(
        1.19687900512017, -0.0528968517574562, -0.0529716355144438,
        -0.0980208811401368, 1.15190312990417, -0.0980434501171241,
        -0.0990297440797205, -0.0989611768448433, 1.15107367264116
    );
    
    val = agx_output_mat * val;
    
    return clamp(val, 0.0, 1.0);
}

// --- 6. Khronos PBR Neutral Tone Mapping ---
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
        result = agx(result);
    } else if (uToneMappingMode == 5) {
        result = khronosPBR(result);
    } else {
        // Fallback to ACES
        result = aces(result);
    }
    
    // Gamma correction
    result = pow(result, vec3(1.0 / 2.2));
    
    FragColor = vec4(result, 1.0);
}
