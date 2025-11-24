#version 330 core
layout(location = 0) in vec4 aPos;
layout(location = 1) in vec3 aColor;
layout(location = 2) in vec2 aLifeMaxLife;

out vec3 vertexColor;
out float alpha;

uniform vec2 uSize;

void main() {
    float a = clamp(aLifeMaxLife.x / max(aLifeMaxLife.y, 0.0001), 0.0, 1.0);
    alpha = a;
    vertexColor = aColor;

    float x = aPos.x / uSize.x * 2.0 - 1.0;
    float y = aPos.y / uSize.y * 2.0 - 1.0;
    gl_Position = vec4(x, y, 0.0, 1.0);

    gl_PointSize = 2.0 + 5.0 * a;
}
