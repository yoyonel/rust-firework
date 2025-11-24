#version 330 core
in vec3 vertexColor;
in float alpha;
out vec4 FragColor;

void main() {
    vec2 uv = gl_PointCoord - vec2(0.5);
    float dist = dot(uv, uv);
    if(dist > 0.25) discard;
    float falloff = smoothstep(0.25, 0.0, dist);
    FragColor = vec4(vertexColor, alpha * falloff);
}
