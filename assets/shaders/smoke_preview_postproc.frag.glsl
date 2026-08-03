#version 330 core
in vec2 vTexCoord;
out vec4 FragColor;
uniform sampler2D uTex;

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
    vec3 col = texture(uTex, vTexCoord).rgb;
    col = khronosPBR(col);
    col = pow(max(col, vec3(0.0)), vec3(1.0 / 2.2));
    FragColor = vec4(col, 1.0);
}
