#version 330 core

in vec2 vTexCoord;
out vec4 FragColor;

uniform sampler2D uSceneTexture;
uniform float uThreshold;

// Calculate luminance using perceptual weights
float luminance(vec3 color) {
    return dot(color, vec3(0.2126, 0.7152, 0.0722));
}

void main() {
    vec3 color = texture(uSceneTexture, vTexCoord).rgb;
    
    // Calculate brightness
    float brightness = luminance(color);
    
    // Extract only bright pixels above threshold
    if (brightness > uThreshold) {
        // Smooth transition around threshold
        float soft = smoothstep(uThreshold, uThreshold + 0.1, brightness);
        FragColor = vec4(color * soft, 1.0);
    } else {
        FragColor = vec4(0.0, 0.0, 0.0, 1.0);
    }
}
