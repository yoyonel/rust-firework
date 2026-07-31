#version 330 core

// Quad geometry inputs (-1.0 to 1.0)
layout(location = 0) in vec2 aQuad;

// Instanced attributes for smoke particles (1 per instance)
layout(location = 1) in vec3 aPosition;       // world position (x, y, z)
layout(location = 2) in float aScale;         // particle scale (expands over lifetime)
layout(location = 3) in float aAlpha;         // opacity (fades 1.0 -> 0.0)
layout(location = 4) in float aRotation;      // random rotation angle in radians
layout(location = 5) in float aIntensity;     // dynamic smoke intensity multiplier
layout(location = 6) in vec3 aColor;          // smoke particle color (RGB)
layout(location = 7) in float aNormalizedAge; // normalized lifetime progress (0.0 -> 1.0)

out vec2 vUV;
out float vAlpha;
out float vIntensity;
out vec3 vColor;
out float vNormalizedAge;

layout (std140) uniform GlobalData {
    vec2 uSize;
    float uTexRatio;
    float uBloomIntensity;
};

void main() {
    vAlpha = aAlpha;
    vIntensity = aIntensity;
    vColor = aColor;
    vNormalizedAge = aNormalizedAge;
    // Map quad vertices (-1.0..1.0) to UV coordinates (0.0..1.0)
    vUV = aQuad * 0.5 + 0.5;

    // Build 2D rotation matrix for random orientation
    float s = sin(aRotation);
    float c = cos(aRotation);
    mat2 rot = mat2(c, -s, s, c);

    // Apply scale and rotation to quad vertex
    vec2 scaledQuad = aQuad * aScale;
    vec2 rotatedQuad = rot * scaledQuad;

    // Translate to world space
    vec2 worldPos = aPosition.xy + rotatedQuad;

    // Screen clip-space transform (-1.0 to 1.0)
    float x = (worldPos.x / uSize.x) * 2.0 - 1.0;
    float y = (worldPos.y / uSize.y) * 2.0 - 1.0;

    gl_Position = vec4(x, y, aPosition.z, 1.0);
}
