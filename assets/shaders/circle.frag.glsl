#version 330 core

in vec4 vColor;
in vec2 vUV;
in float vRadius;
in float vThickness;

layout(location = 0) out vec4 FragColor;
layout(location = 1) out vec4 BrightColor;

void main() {
    // UV is in [-0.5, 0.5] space. Distance from center is in [0.0, 0.5] space.
    float dist = length(vUV);

    if (dist > 0.5) {
        discard;
    }

    if (vThickness > 0.0) {
        // Wireframe circle
        // The thickness in UV space is: thickness_pixels / (2.0 * radius_pixels)
        float uv_thickness = vThickness / (2.0 * vRadius);
        if (dist < 0.5 - uv_thickness) {
            discard;
        }
    }

    FragColor = vColor;
    BrightColor = vec4(0.0);
}
