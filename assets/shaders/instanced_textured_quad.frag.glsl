#version 330 core

in vec3 vColor;
in float vAlpha;
in vec2 vUV;
in float vBrightness;  // HDR multiplier from vertex shader

layout(location = 0) out vec4 FragColor;
layout(location = 1) out vec4 BrightColor;

uniform sampler2D uTexture;

void main() {
    if (vAlpha <= 0.0) discard;
    vec4 texColor = texture(uTexture, vUV);
    vec4 baseColor = vec4(vColor, vAlpha) * texColor;
    
    // MRT Output
    // Location 0: Scene Color
    FragColor = baseColor;
    
    // Location 1: Bloom Color
    BrightColor = baseColor;
    BrightColor.rgb *= vBrightness;
}
