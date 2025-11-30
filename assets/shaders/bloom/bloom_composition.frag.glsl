#version 330 core

in vec2 vTexCoord;
out vec4 FragColor;

uniform sampler2D uSceneTexture;
uniform sampler2D uBloomTexture;
uniform float uBloomIntensity;

// Simple Reinhard tone mapping
vec3 toneMap(vec3 color) {
    return color / (color + vec3(1.0));
}

// ACES filmic tone mapping (optional, more cinematic)
vec3 acesToneMap(vec3 color) {
    const float a = 2.51;
    const float b = 0.03;
    const float c = 2.43;
    const float d = 0.59;
    const float e = 0.14;
    return clamp((color * (a * color + b)) / (color * (c * color + d) + e), 0.0, 1.0);
}

void main() {
    vec3 sceneColor = texture(uSceneTexture, vTexCoord).rgb;
    vec3 bloomColor = texture(uBloomTexture, vTexCoord).rgb;
    
    // Additive blending with intensity control
    vec3 result = sceneColor + bloomColor * uBloomIntensity;
    
    // Apply tone mapping to convert HDR to LDR
    // result = toneMap(result);
    result = acesToneMap(result);
    
    // Gamma correction
    result = pow(result, vec3(1.0 / 2.2));
    
    FragColor = vec4(result, 1.0);
}
