#version 330 core

in vec2 vTexCoord;

// Multiple render targets - one for each tone mapping
layout(location = 0) out vec4 FragColor0; // Reinhard
layout(location = 1) out vec4 FragColor1; // Reinhard Extended
layout(location = 2) out vec4 FragColor2; // ACES
layout(location = 3) out vec4 FragColor3; // Uncharted 2
layout(location = 4) out vec4 FragColor4; // Khronos PBR

uniform sampler2D uSceneTexture;
uniform sampler2D uBloomTexture;
uniform float uBloomIntensity;

// --- Tone Mapping Functions ---

vec3 reinhard(vec3 color) {
    return color / (color + vec3(1.0));
}

vec3 reinhardExtended(vec3 color) {
    float whitePoint = 4.0;
    vec3 numerator = color * (vec3(1.0) + (color / (whitePoint * whitePoint)));
    return numerator / (vec3(1.0) + color);
}

vec3 aces(vec3 color) {
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), 0.0, 1.0);
}

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
    vec3 hdrColor = sceneColor + bloomColor * uBloomIntensity;
    
    // Apply each tone mapping and gamma correction
    vec3 result0 = pow(reinhard(hdrColor), vec3(1.0 / 2.2));
    vec3 result1 = pow(reinhardExtended(hdrColor), vec3(1.0 / 2.2));
    vec3 result2 = pow(aces(hdrColor), vec3(1.0 / 2.2));
    vec3 result3 = pow(uncharted2(hdrColor), vec3(1.0 / 2.2));
    vec3 result4 = pow(khronosPBR(hdrColor), vec3(1.0 / 2.2));
    
    // Output to multiple render targets
    FragColor0 = vec4(result0, 1.0);
    FragColor1 = vec4(result1, 1.0);
    FragColor2 = vec4(result2, 1.0);
    FragColor3 = vec4(result3, 1.0);
    FragColor4 = vec4(result4, 1.0);
}
