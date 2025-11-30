#version 330 core
in vec3 vertexColor;
in float alpha;
in float vBrightness;  // HDR multiplier from vertex shader
layout(location = 0) out vec4 FragColor;
layout(location = 1) out vec4 BrightColor;

void main() {
    vec2 uv = gl_PointCoord - vec2(0.5);
    float dist = dot(uv, uv);
    if(dist > 0.25) discard;    
    float falloff = smoothstep(0.25, 0.0, dist);
    
    // MRT Output
    // Location 0: Scene Color (Base color)
    FragColor = vec4(vertexColor, alpha * falloff);
    
    // Location 1: Brightness/Bloom Color (Controlled by vBrightness)
    // vBrightness acts as an emission multiplier. 
    // If vBrightness is 0, no bloom. If high, strong bloom.
    BrightColor = vec4(vertexColor * vBrightness, alpha * falloff);
}
