#version 330 core

// Per-vertex geometry:
// - Ring pass  (uMode == 0): aQuad ∈ [-0.5, 0.5]²  (TRIANGLE_STRIP quad)
// - Beam pass  (uMode == 1): aQuad ∈ {(0,0), (1,0)} (GL_LINES endpoints)
layout(location = 0) in vec2 aQuad;

// Per-instance (VertexAttribDivisor = 1)
layout(location = 1) in vec2  aPos;       // world position of the audio event
layout(location = 2) in float aAge;       // age in seconds  [0 .. aTtl]
layout(location = 3) in float aTtl;       // total lifetime in seconds
layout(location = 4) in float aKind;      // 0.0 = Launch (green), 1.0 = Explosion (red)
layout(location = 5) in vec2  aListener;  // listener world position

out vec2  vUV;     // ring: local [-0.5..0.5] coord; beam: raw aQuad
out float vAge;
out float vTtl;
out float vKind;

// Draw-call mode (set via glUniform1i before each instanced draw)
// 0 = ring quad, 1 = beam line
uniform int uMode;

// Screen dimensions – shared UBO, binding point 0 (same as all other shaders)
layout(std140) uniform GlobalData {
    vec2  uSize;
    float uTexRatio;
    float uBloomIntensity;
};

// Peak visual radius of a ripple ring, in pixels
const float MAX_RADIUS = 80.0;

void main() {
    vUV  = aQuad;
    vAge = aAge;
    vTtl = aTtl;
    vKind = aKind;

    vec2 world_pos;

    if (uMode == 1) {
        // ── Beam pass ─────────────────────────────────────────────────────
        // aQuad.x == 0.0 → event origin, aQuad.x == 1.0 → listener
        world_pos = mix(aPos, aListener, aQuad.x);
    } else {
        // ── Ring pass ─────────────────────────────────────────────────────
        float t      = clamp(aAge / aTtl, 0.0, 1.0);
        float radius = t * MAX_RADIUS;
        // Scale the [-0.5, 0.5] quad to cover the full ring diameter
        world_pos = aPos + aQuad * (2.0 * max(radius, 4.0));
    }

    // Pixel-space → NDC  (origin bottom-left, Y increases upward)
    float x =  world_pos.x / uSize.x * 2.0 - 1.0;
    float y =  world_pos.y / uSize.y * 2.0 - 1.0;
    gl_Position = vec4(x, y, 0.0, 1.0);
}
