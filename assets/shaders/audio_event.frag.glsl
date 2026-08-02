#version 330 core

in vec2  vUV;
in float vAge;
in float vTtl;
in float vKind;

layout(location = 0) out vec4 FragColor;
layout(location = 1) out vec4 BrightColor;

// 0 = ring pass, 1 = beam pass  (matches vertex shader uMode)
uniform int uMode;

// ── Color palette ─────────────────────────────────────────────────────────
const vec3 LAUNCH_COLOR    = vec3(0.15, 1.00, 0.40);  // bright green
const vec3 EXPLOSION_COLOR = vec3(1.00, 0.35, 0.05);  // orange-red
const float RING_WIDTH     = 0.08;

void main() {
    // Normalised lifetime [0..1]
    float t = clamp(vAge / vTtl, 0.0, 1.0);

    // Fast attack, smooth decay (safe smoothstep usage with edge0 < edge1)
    float attack = smoothstep(0.0, 0.05, t);
    float decay = 1.0 - smoothstep(0.5, 1.0, t);
    float alpha = attack * decay;

    vec3 color = mix(LAUNCH_COLOR, EXPLOSION_COLOR, step(0.5, vKind));

    if (uMode == 1) {
        // ── Beam pass: line fading toward the listener ────────────────────
        // vUV.x == 0 at event origin, 1 at listener
        float beam_alpha = alpha * 0.55 * (1.0 - clamp(vUV.x, 0.0, 1.0));
        if (beam_alpha < 0.01) discard;
        FragColor   = vec4(color, beam_alpha);
        BrightColor = vec4(0.0);
        return;
    }

    // ── Ring pass: SDF dual-wavefront rings ───────────────────────────────
    float dist = length(vUV);
    if (dist > 0.5) discard;

    // Primary (outer) ring - safe smoothstep usage
    float outer = 0.5;
    float inner = outer - RING_WIDTH;
    float ring  = smoothstep(inner - 0.01, inner, dist) *
                  (1.0 - smoothstep(outer, outer + 0.01, dist));

    // Secondary (inner, dimmer) ring trailing at ~55 % of primary radius
    float outer2 = 0.5 * 0.55;
    float inner2 = outer2 - RING_WIDTH * 0.65;
    float ring2  = smoothstep(inner2 - 0.01, inner2, dist) *
                   (1.0 - smoothstep(outer2, outer2 + 0.01, dist)) * 0.40;

    float ring_total = clamp(ring + ring2, 0.0, 1.0);

    // Very soft center glow
    float glow = (1.0 - smoothstep(0.0, 0.5, dist)) * 0.10;

    float final_alpha = (ring_total + glow) * alpha;
    if (final_alpha < 0.005) discard;

    vec3 final_rgb = color + glow * color * 0.4;

    FragColor   = vec4(final_rgb, final_alpha);
    BrightColor = vec4(0.0);  // post-bloom draw, BrightColor unused
}
