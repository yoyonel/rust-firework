#version 330 core

in vec3 vColor;
in float vAlpha;
in vec2 vUV;
in float vBrightness;  // HDR multiplier from vertex shader

out vec4 FragColor;

uniform sampler2D uTexture;

void main() {
    if (vAlpha <= 0.0) discard;
    vec4 texColor = texture(uTexture, vUV);
    FragColor = vec4(vColor, vAlpha) * texColor;
    
    // Apply HDR brightness multiplier
    FragColor.rgb *= vBrightness;
}
