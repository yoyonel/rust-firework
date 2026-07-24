#version 330 core

// Per-vertex: quad position OR line endpoints.
// - For the ring quad pass: aQuad ∈ [-0.5, 0.5]²
// - For the beam line pass: aQuad ∈ {(0,0), (1,0)} — x selects [aPos → aListener]
layout(location = 0) in vec2 aQuad;

// Per-instance (divisor = 1)
layout(location = 1) in vec2  aPos;       // world position of the event
layout(location = 2) in float aAge;       // age in seconds [0 .. aTtl]
layout(location = 3) in float aTtl;       // total lifetime in seconds
layout(location = 4) in float aKind;      // 0.0 = Launch, 1.0 = Explosion
layout(location = 5) in vec2  aListener;  // listener world position

// Outputs to fragment shader
out vec2  vUV;
out float vAge;
out float vTtl;
out float vKind;
// Beam pass uses this to distinguish beam pixels from ring pixels
out float vIsBeam;  // 0.0 = ring quad, 1.0 = beam line

// Screen dimensions from the shared UBO (binding point 0, same as particles)
layout(std140) uniform GlobalData {
    vec2  uSize;
    float uTexRatio;
    float uBloomIntensity;
};

// Maximum visual radius of a ripple ring in pixels at peak expansion
const float MAX_RADIUS = 80.0;

void main() {
    vUV    = aQuad;
    vAge   = aAge;
    vTtl   = aTtl;
    vKind  = aKind;

    // Detect beam vs ring pass by checking if aQuad.x is 0 or 1
    // (ring quad is [-0.5..0.5], beam line verts are exactly 0.0 and 1.0)
    vIsBeam = step(0.9, abs(aQuad.x) + abs(aQuad.y));

    vec2 world_pos;

    if (vIsBeam > 0.5) {
        // BEAM PASS: lerp from event position to listener position
        // aQuad.x == 0.0 → event origin, aQuad.x == 1.0 → listener
        world_pos = mix(aPos, aListener, aQuad.x);
    } else {
        // RING PASS: expand the quad centered on the event position.
        // t rises from 0→1 over the event lifetime.
        float t      = clamp(aAge / aTtl, 0.0, 1.0);
        float radius = t * MAX_RADIUS;
        world_pos = aPos + aQuad * (2.0 * max(radius, 4.0));
    }

    float x =  world_pos.x / uSize.x * 2.0 - 1.0;
    float y =  world_pos.y / uSize.y * 2.0 - 1.0;
    gl_Position = vec4(x, y, 0.0, 1.0);
}
