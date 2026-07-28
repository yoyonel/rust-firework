#version 330 core

// Quad vertices (-0.5 to 0.5)
layout(location = 0) in vec2 aQuad;

// Instanced attributes
layout(location = 1) in vec2 aCenter;
layout(location = 2) in float aRadius;
layout(location = 3) in vec4 aColor;
layout(location = 4) in float aThickness;

out vec4 vColor;
out vec2 vUV; // -0.5 to 0.5 coordinate inside the quad
out float vRadius;
out float vThickness;

layout (std140) uniform GlobalData {
    vec2 uSize;
    float uTexRatio;
    float uBloomIntensity;
};

void main() {
    vColor = aColor;
    vUV = aQuad; // Pass raw quad coord (-0.5 to 0.5)
    vRadius = aRadius;
    vThickness = aThickness;

    // Position of the vertex in world pixels
    // Since aQuad ranges from -0.5 to 0.5, multiplying it by 2.0 * aRadius
    // scales the quad to exactly cover the diameter of the circle!
    vec2 world_pos = aCenter + aQuad * (2.0 * aRadius);

    // Convert to NDC clip space
    float x = world_pos.x / uSize.x * 2.0 - 1.0;
    float y = world_pos.y / uSize.y * 2.0 - 1.0;
    gl_Position = vec4(x, y, 0.0, 1.0);
}
