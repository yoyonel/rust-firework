#version 330 core

in vec2  vUV;
in float vAge;
in float vTtl;
in float vKind;
in float vIsBeam;

layout(location = 0) out vec4 FragColor;
layout(location = 1) out vec4 BrightColor;

// Ring expansion config
const float RING_WIDTH  = 0.08;   // fraction of half-extent in UV space

// ── Color palette ──────────────────────────────────────────────────────────
const vec3 LAUNCH_COLOR    = vec3(0.15, 1.00, 0.40);  // bright green
const vec3 EXPLOSION_COLOR = vec3(1.00, 0.35, 0.05);  // orange-red

void main() {
    // Normalized lifetime progress [0..1]
    float t     = clamp(vAge / vTtl, 0.0, 1.0);
    // Global alpha: fast attack (first 5%), smooth decay
    float alpha = smoothstep(0.0, 0.05, t) * smoothstep(1.0, 0.5, t);

    vec3 color = mix(LAUNCH_COLOR, EXPLOSION_COLOR, step(0.5, vKind));

    if (vIsBeam > 0.5) {
        // ── BEAM PASS: simple line with fading alpha ───────────────────────
        // The line endpoints are at event origin and listener.
        // Along the line, fade more strongly near the listener end
        // (vUV.x is 0.0 at origin, 1.0 at listener after beam lerp).
        // But in the beam pass, vUV is actually the raw vertex coord
        // (0,0) or (1,0), so use aQuad.x via vUV.x.
        float beam_alpha = alpha * 0.6 * smoothstep(1.0, 0.0, vUV.x);
        if (beam_alpha < 0.01) discard;
        FragColor   = vec4(color, beam_alpha);
        BrightColor = vec4(0.0);
        return;
    }

    // ── RING PASS: SDF annular rings ───────────────────────────────────────
    // Distance from center in [0..0.5] UV space
    float dist = length(vUV);
    if (dist > 0.5) discard;

    // Primary ring: thin band near the outer edge of the quad
    float outer = 0.5;
    float inner = outer - RING_WIDTH;
    float ring  = smoothstep(inner - 0.01, inner,       dist) *
                  smoothstep(outer + 0.01, outer,       dist);

    // Secondary ring: inner wavefront trailing at 55% of the outer radius
    float outer2 = 0.5 * 0.55;
    float inner2 = outer2 - RING_WIDTH * 0.65;
    float ring2  = smoothstep(inner2 - 0.01, inner2,   dist) *
                   smoothstep(outer2 + 0.01, outer2,   dist) * 0.40;

    float ring_total = clamp(ring + ring2, 0.0, 1.0);

    // Soft center glow (very subtle, widens the visible indicator)
    float glow = smoothstep(0.5, 0.0, dist) * 0.12;

    float final_alpha = (ring_total + glow) * alpha;
    if (final_alpha < 0.005) discard;

    // Slight color boost for center glow vs ring
    vec3 final_rgb = color + glow * color * 0.5;

    FragColor   = vec4(final_rgb, final_alpha);
    // No bloom — we draw to the default framebuffer post-bloom-composite.
    // Keep BrightColor at zero to avoid wasting bandwidth.
    BrightColor = vec4(0.0);
}
