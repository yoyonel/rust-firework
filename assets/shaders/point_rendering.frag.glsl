#version 330 core
in vec3 vertexColor;
in float alpha;
in float vBrightness;  // HDR multiplier from vertex shader
out vec4 FragColor;

void main() {
    vec2 uv = gl_PointCoord - vec2(0.5);
    float dist = dot(uv, uv);
    if(dist > 0.25) discard;    
    float falloff = smoothstep(0.25, 0.0, dist);
    
    // Apply HDR brightness multiplier (can exceed 1.0 for bloom)
    vec3 hdrColor = vertexColor * vBrightness;
    FragColor = vec4(hdrColor, alpha * falloff);
}
