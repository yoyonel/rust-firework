#version 330 core
layout(location = 0) in vec2 pos;
layout(location = 1) in vec3 color;
layout(location = 2) in vec4 lifeData;  // life, max_life, size, angle
layout(location = 3) in float brightness;  // HDR multiplier

out vec3 vertexColor;
out float alpha;
out float vBrightness;  // Pass brightness to fragment shader

uniform vec2 uSize;

void main() {
    float a = clamp(lifeData.x / max(lifeData.y, 0.0001), 0.0, 1.0);
    alpha = a;
    vertexColor = color;
    vBrightness = brightness;

    float x = pos.x / uSize.x * 2.0 - 1.0;
    float y = pos.y / uSize.y * 2.0 - 1.0;
    gl_Position = vec4(x, y, 0.0, 1.0);

    gl_PointSize = lifeData.z;
}
