#version 330 core

in vec2 vUV;
in float vAlpha;
in float vIntensity;
in vec3 vColor;
in float vNormalizedAge;
in vec2 vWorldPos;

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
    // 1. Early discard for transparent or unlit instances
    if (vAlpha <= 0.001 || vIntensity <= 0.001) {
        discard;
    }

    // 2. Early quad corner culling (radial mask r > 0.5)
    // Eliminates fragment shading & texture lookups for quad corners (~21.5% fillrate saved)
    vec2 centerOffset = vUV - vec2(0.5);
    if (dot(centerOffset, centerOffset) > 0.25) {
        discard;
    }

    // 3. Early Alpha Erosion / Dissolve Discard
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

    // 4. Flow map distortion and smoke texture sampling
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

    // 5. Early alpha discard on smoke texture alpha
    float finalAlpha = smokeTex.a * vAlpha;
    if (finalAlpha <= 0.001) {
        discard;
    }

    vec3 finalColor = smokeTex.rgb * vColor;

    // 6. Glowing burn edge effect along erosion seam (reusing noiseVal already sampled)
    if (u_ErosionEnabled && u_ErosionScale > 0.001) {
        if (noiseVal < erosionThreshold + u_ErosionEdgeWidth) {
            float edgeFactor = (noiseVal - erosionThreshold) / max(0.0001, u_ErosionEdgeWidth);
            finalColor = mix(u_ErosionEdgeColor, finalColor, edgeFactor);
            finalAlpha = min(1.0, finalAlpha * 1.5);
        }
    }

    // 7. Volumetric In-Scattering (Participating Media Lighting)
    if (u_ScatteringIntensity > 0.001) {
        vec3 scatteredLight = u_AmbientLight.rgb + vec3(u_AmbientLight.a);
        for (int i = 0; i < u_NumActiveLights; ++i) {
            vec2 lightPos = u_Lights[i].position_radius.xy;
            float radius = u_Lights[i].position_radius.w;
            vec2 toLight = lightPos - vWorldPos;
            float distSq = dot(toLight, toLight);
            float radiusSq = radius * radius;

            if (distSq < radiusSq) {
                float dist = sqrt(distSq);
                float atten = 1.0 - (dist / radius);
                atten = atten * atten; // Smooth quadratic falloff

                vec3 lightCol = u_Lights[i].color_intensity.rgb;
                float intensity = u_Lights[i].color_intensity.w;

                // Anisotropic forward scattering approximation (Schlick/Mie phase)
                float cosTheta = dot(normalize(toLight), vec2(0.0, 1.0));
                float phase = 1.0 + 0.3 * cosTheta;

                scatteredLight += lightCol * (intensity * atten * phase * u_ScatteringIntensity);
            }
        }
        // Balanced in-scattering boost: illuminates smoke realistically without blowing out highlights
        finalColor += finalColor * clamp(scatteredLight * 0.25, vec3(0.0), vec3(1.2));
    }

    FragColor = vec4(finalColor * vIntensity, finalAlpha * vIntensity);

    // Smoke is non-emissive volumetric dust; keep bright bloom attachment zero to avoid blinding wash-out
    BrightColor = vec4(0.0, 0.0, 0.0, 0.0);
}
