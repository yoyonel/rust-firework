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
out vec2 vWorldPos;
out vec3 vScatteredLight;

layout (std140) uniform GlobalData {
    vec2 uSize;
    float uTexRatio;
    float uBloomIntensity;
};

struct PointLight {
    vec4 position_radius; // xyz = world pos, w = radius
    vec4 color_intensity; // rgb = color, w = intensity
};

layout (std140) uniform LightingBlock {
    PointLight u_Lights[16];
    vec4 u_AmbientLight; // rgb = ambient tint, a = global flash intensity
    int u_NumActiveLights;
    float u_ScatteringIntensity;
};

uniform int u_RenderMask;
uniform int u_UseLut;
uniform sampler2D u_LightFalloffLut;

void main() {
    // 1. Early clipping for dead or fully transparent particles (hardware rasterizer bypass)
    if (aAlpha <= 0.001 || aIntensity <= 0.001) {
        gl_Position = vec4(2.0, 2.0, 2.0, 1.0);
        return;
    }

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
    vWorldPos = worldPos;

    // 2. Per-vertex volumetric in-scattering (computed at vertex stage, bypassed during mask pass)
    vec3 scatteredLight = vec3(0.0);
    if (u_RenderMask == 0 && u_ScatteringIntensity > 0.001) {
        // Subtle distant atmospheric flash only (2% max) to avoid bleaching entire screen trails
        scatteredLight = (u_AmbientLight.rgb + vec3(u_AmbientLight.a)) * 0.02;
        if (u_UseLut != 0) {
            for (int i = 0; i < u_NumActiveLights; ++i) {
                vec2 lightPos = u_Lights[i].position_radius.xy;
                float radius = u_Lights[i].position_radius.w;
                vec2 toLight = lightPos - worldPos;

                vec2 lutUV = toLight * (0.5 / radius) + 0.5;
                float falloff = textureLod(u_LightFalloffLut, lutUV, 0.0).r;

                vec3 lightCol = u_Lights[i].color_intensity.rgb;
                float intensity = u_Lights[i].color_intensity.w;

                scatteredLight += lightCol * (intensity * falloff * u_ScatteringIntensity);
            }
        } else {
            for (int i = 0; i < u_NumActiveLights; ++i) {
                vec2 lightPos = u_Lights[i].position_radius.xy;
                float radius = u_Lights[i].position_radius.w;
                vec2 toLight = lightPos - worldPos;
                float distSq = dot(toLight, toLight);
                float radiusSq = radius * radius;

                if (distSq < radiusSq) {
                    float dist = sqrt(distSq);
                    float atten = 1.0 - (dist / radius);
                    atten = atten * atten; // Smooth quadratic falloff identical to canon

                    vec3 lightCol = u_Lights[i].color_intensity.rgb;
                    float intensity = u_Lights[i].color_intensity.w;

                    // Anisotropic forward scattering approximation (Schlick/Mie phase)
                    float cosTheta = toLight.y / max(0.0001, dist);
                    float phase = 1.0 + 0.3 * cosTheta;

                    scatteredLight += lightCol * (intensity * atten * phase * u_ScatteringIntensity);
                }
            }
        }
    }
    vScatteredLight = scatteredLight;

    // Screen clip-space transform (-1.0 to 1.0)
    float x = (worldPos.x / uSize.x) * 2.0 - 1.0;
    float y = (worldPos.y / uSize.y) * 2.0 - 1.0;

    gl_Position = vec4(x, y, aPosition.z, 1.0);
}
