#version 330 core

in vec4 vColor;
in vec2 vUV;
in float vRadius;
in float vThickness;
in vec2 vWorldPos;

layout(location = 0) out vec4 FragColor;
layout(location = 1) out vec4 BrightColor;

void main() {
    // Negative thickness denotes dashed bounding box (pointillé) mode
    if (vThickness < 0.0) {
        float dash_len = max(-vThickness, 4.0);
        // Stipple pattern in screen pixel space along axis-aligned line loops
        if (mod(floor((vWorldPos.x + vWorldPos.y + 100000.0) / dash_len), 2.0) < 1.0) {
            discard;
        }
        FragColor = vColor;
        BrightColor = vec4(0.0);
        return;
    }

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
