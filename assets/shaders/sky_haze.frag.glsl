#version 330 core

in vec2 vTexCoord;

layout(location = 0) out vec4 FragColor;
layout(location = 1) out vec4 BrightColor;

layout (std140) uniform GlobalData {
    vec2 uSize;         // window width, height in pixels
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

uniform float u_HazeIntensity;
uniform float u_AmbientFlash;
uniform float u_HazeFalloff;

void main() {
    // 1. World coordinates matching 2D simulation viewport (Y goes upward, 0 at bottom)
    vec2 worldPos = vTexCoord * uSize;

    // 2. Base deep obsidian midnight sky (pure rich dark night)
    float horizonFactor = 1.0 - vTexCoord.y;
    vec3 baseSky = mix(
        vec3(0.0, 0.0001, 0.0003),
        vec3(0.0003, 0.0005, 0.0010),
        horizonFactor * horizonFactor
    );

    // 3. Faint detonation flash across distant atmosphere (subtle, non-blinding, deep midnight)
    vec3 ambientGlow = (u_AmbientLight.rgb + vec3(u_AmbientLight.a * 0.3)) * (u_AmbientFlash * 0.002);
    vec3 skyLight = baseSky + ambientGlow;

    // 4. Localized atmospheric haze glow around active detonations and rocket heads
    if (u_HazeIntensity > 0.001) {
        for (int i = 0; i < u_NumActiveLights; ++i) {
            vec2 lightPos = u_Lights[i].position_radius.xy;
            float lightRadius = u_Lights[i].position_radius.w * 0.85; // Focused atmospheric envelope
            vec2 toLight = lightPos - worldPos;
            float distSq = dot(toLight, toLight);
            float radiusSq = lightRadius * lightRadius;

            if (distSq < radiusSq) {
                float dist = sqrt(distSq);
                float normDist = dist / lightRadius;

                // Dynamic exponential falloff: intense near detonation, fades rapidly into deep darkness
                float falloffExponent = max(0.5, u_HazeFalloff);
                float atten = exp(-falloffExponent * normDist) * (1.0 - normDist);

                vec3 lightCol = u_Lights[i].color_intensity.rgb;
                float intensity = u_Lights[i].color_intensity.w;

                skyLight += lightCol * (intensity * atten * u_HazeIntensity * 0.035);
            }
        }
    }

    FragColor = vec4(skyLight, 1.0);

    // Never inject background sky into bloom pass to preserve pure deep night contrast
    BrightColor = vec4(0.0, 0.0, 0.0, 0.0);
}
