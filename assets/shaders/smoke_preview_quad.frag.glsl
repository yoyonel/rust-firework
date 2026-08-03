#version 330 core
in vec2 vTexCoord;
out vec4 FragColor;
uniform sampler2D uTex;
void main() {
    vec4 texColor = texture(uTex, vTexCoord);
    if (texColor.a < 0.05) discard;
    FragColor = texColor;
}
