#version 330 core

in vec2 vUV;
in float vAlpha;
in float vIntensity;
in vec3 vColor;
in float vNormalizedAge;
in vec2 vWorldPos;
in vec3 vScatteredLight;

layout(location = 0) out vec4 FragColor;
layout(location = 1) out vec4 BrightColor;

uniform sampler2D u_SmokeTexture;
uniform sampler2D u_FlowMap;
uniform sampler2D u_NoiseTexture;

uniform float u_FlowDistortionStrength;
uniform float u_FlowAnimationSpeed;

uniform bool u_ErosionEnabled;
uniform float u_ErosionScale;
uniform float u_ErosionEdgeWidth;
uniform vec3 u_ErosionEdgeColor;

uniform int u_RenderMask;

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

void main() {
    // 1. Early discard for transparent or unlit instances (safety fallback)
    if (vAlpha <= 0.001 || vIntensity <= 0.001) {
        discard;
    }

    // 2. Early Alpha Erosion / Dissolve Discard
    // Sample noise texture FIRST to immediately discard eroded fragments before sampling flow map or smoke texture
    float noiseVal = 1.0;
    float erosionThreshold = 0.0;
    if (u_ErosionEnabled && u_ErosionScale > 0.001) {
        noiseVal = texture(u_NoiseTexture, vUV).r;
        erosionThreshold = clamp(vNormalizedAge * u_ErosionScale, 0.0, 1.0);
        if (noiseVal < erosionThreshold) {
            discard;
        }
    }

    // 3. Flow map distortion and smoke texture sampling
    vec4 smokeTex;
    if (u_FlowDistortionStrength > 0.001) {
        vec2 flow = texture(u_FlowMap, vUV).rg * 2.0 - 1.0;
        flow *= u_FlowDistortionStrength;

        float t = vNormalizedAge * u_FlowAnimationSpeed * 5.0;
        float blend = abs((fract(t) - 0.5) * 2.0);

        if (blend < 0.05) {
            float phase0 = fract(t);
            smokeTex = texture(u_SmokeTexture, vUV + flow * phase0);
        } else if (blend > 0.95) {
            float phase1 = fract(t + 0.5);
            smokeTex = texture(u_SmokeTexture, vUV + flow * phase1);
        } else {
            float phase0 = fract(t);
            float phase1 = fract(t + 0.5);
            vec4 tex0 = texture(u_SmokeTexture, vUV + flow * phase0);
            vec4 tex1 = texture(u_SmokeTexture, vUV + flow * phase1);
            smokeTex = mix(tex0, tex1, blend);
        }
    } else {
        smokeTex = texture(u_SmokeTexture, vUV);
    }

    // 4. Early alpha discard on smoke texture alpha
    float finalAlpha = smokeTex.a * vAlpha;
    if (finalAlpha <= 0.001) {
        discard;
    }

    // 5. Glowing burn edge alpha boost along erosion seam (reusing noiseVal already sampled)
    if (u_ErosionEnabled && u_ErosionScale > 0.001) {
        if (noiseVal < erosionThreshold + u_ErosionEdgeWidth) {
            finalAlpha = min(1.0, finalAlpha * 1.5);
        }
    }

    // Screen-space smoke backlight mask pass (§4.1 ADR)
    // Directly output actual visible screen alpha (finalAlpha * vIntensity) and bypass all color/scattering logic
    if (u_RenderMask != 0) {
        float maskAlpha = finalAlpha * vIntensity;
        FragColor = vec4(maskAlpha, 0.0, 0.0, 1.0);
        BrightColor = vec4(0.0);
        return;
    }

    vec3 finalColor = smokeTex.rgb * vColor;

    // Glowing burn edge color along erosion seam
    if (u_ErosionEnabled && u_ErosionScale > 0.001) {
        if (noiseVal < erosionThreshold + u_ErosionEdgeWidth) {
            float edgeFactor = (noiseVal - erosionThreshold) / max(0.0001, u_ErosionEdgeWidth);
            finalColor = mix(u_ErosionEdgeColor, finalColor, edgeFactor);
        }
    }

    // 6. Volumetric In-Scattering (interpolated smoothly from vertices, zero per-pixel loop)
    // Retains exact canonical soot modulation (factor 0.25, zero additive blowout)
    if (u_ScatteringIntensity > 0.001) {
        finalColor += finalColor * clamp(vScatteredLight * 0.25, vec3(0.0), vec3(1.2));
    }

    FragColor = vec4(finalColor * vIntensity, finalAlpha * vIntensity);

    // Smoke is non-emissive volumetric dust; keep bright bloom attachment zero to avoid blinding wash-out
    BrightColor = vec4(0.0, 0.0, 0.0, 0.0);
}
