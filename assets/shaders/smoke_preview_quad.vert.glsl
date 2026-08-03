#version 330 core
layout(location = 0) in vec2 aPos;
layout(location = 1) in vec2 aTexCoord;
out vec2 vTexCoord;
uniform vec4 uRect;
uniform vec2 uSize;
uniform float uRotZ;
void main() {
    vTexCoord = aTexCoord;
    vec2 half_extent = uRect.zw * 0.5;
    vec2 local_pos = aPos * half_extent;
    float cos_a = cos(uRotZ);
    float sin_a = sin(uRotZ);
    vec2 rotated_local = vec2(
        local_pos.x * cos_a - local_pos.y * sin_a,
        local_pos.x * sin_a + local_pos.y * cos_a
    );
    vec2 world_pos = uRect.xy + rotated_local;
    vec2 ndc = (world_pos / uSize) * 2.0 - 1.0;
    gl_Position = vec4(ndc, 0.0, 1.0);
}
