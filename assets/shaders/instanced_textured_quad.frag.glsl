#version 330 core

in vec3 vColor;
in float vAlpha;
in vec2 vUV;

out vec4 FragColor;

uniform sampler2D uTexture;

void main() {
    if (vAlpha <= 0.0) discard;
    FragColor = vec4(vColor, vAlpha) * texture(uTexture, vUV);
}
