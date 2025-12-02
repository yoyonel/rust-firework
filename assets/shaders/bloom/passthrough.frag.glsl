#version 330 core

in vec2 vTexCoord;
out vec4 FragColor;

uniform sampler2D uTexture;

void main() {
    // Just display the texture as-is (already tone-mapped and gamma corrected)
    FragColor = texture(uTexture, vTexCoord);
}
