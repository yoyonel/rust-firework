#version 330 core

in vec2 vTexCoord;
out vec4 FragColor;

uniform sampler2D uTexture;
uniform vec2 uDirection; // (1, 0) for horizontal, (0, 1) for vertical

// Optimized Gaussian blur using linear texture sampling
// This reduces the number of texture fetches from 9 to 5
// by exploiting bilinear filtering

void main() {
    vec2 texelSize = 1.0 / vec2(textureSize(uTexture, 0));
    vec2 offset = uDirection * texelSize;
    
    // 5-tap Gaussian kernel weights (optimized for linear sampling)
    // Original 9-tap: [0.0162, 0.0540, 0.1216, 0.1945, 0.2270, 0.1945, 0.1216, 0.0540, 0.0162]
    // Optimized to 5-tap by combining adjacent samples
    
    vec3 result = vec3(0.0);
    
    // Center sample
    result += texture(uTexture, vTexCoord).rgb * 0.2270;
    
    // First pair (combines samples at offset 1 and 2)
    result += texture(uTexture, vTexCoord + offset * 1.3846153846).rgb * 0.3162162162;
    result += texture(uTexture, vTexCoord - offset * 1.3846153846).rgb * 0.3162162162;
    
    // Second pair (combines samples at offset 3 and 4)
    result += texture(uTexture, vTexCoord + offset * 3.2307692308).rgb * 0.0702702703;
    result += texture(uTexture, vTexCoord - offset * 3.2307692308).rgb * 0.0702702703;
    
    FragColor = vec4(result, 1.0);
}
