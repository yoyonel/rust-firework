#version 330 core

// === Quad unité (4 sommets pour TRIANGLE_STRIP)
layout(location = 0) in vec2 aQuad;

// === Attributs instanciés (1 par particule)
layout(location = 1) in vec2 aPos;
layout(location = 2) in vec3 aColor;
layout(location = 3) in vec4 aLifeMaxLifeSizeAngle;
layout(location = 4) in float aBrightness;  // HDR multiplier

out vec3 vColor;
out float vAlpha;
out vec2 vUV;
out float vBrightness;  // Pass brightness to fragment shader

uniform vec2 uSize;
uniform float uTexRatio;

mat3 build_world_matrix(float size, float angle) {
    // Position du sommet quad dans l'espace clip (avec taille)
    float scale = size * (2.0 + 5.0 * vAlpha);
    
    float sx = scale * uTexRatio;
    float sy = scale * 1.0;            

    mat3 mat_scale = mat3(
        sx, 0.0, 0.0,
        0.0, sy, 0.0,
        0.0, 0.0, 1.0
    );

    float s = sin(angle);
    float c = cos(angle);
    mat3 mat_rotation = mat3(
        c, -s, 0.0,
        s,  c, 0.0,
        0.0, 0.0, 1.0
    );
    
    mat3 mat_translation = mat3(
        1.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        aPos.x, aPos.y, 1.0
    );

    return mat_translation * mat_rotation * mat_scale;
}

void main() {
    float life = aLifeMaxLifeSizeAngle.x;
    float max_life = aLifeMaxLifeSizeAngle.y;
    float size = aLifeMaxLifeSizeAngle.z;
    float angle = aLifeMaxLifeSizeAngle.w;

    // Ratio de vie (comme avant)
    vAlpha = clamp(life / max(max_life, 0.0001), 0.0, 1.0);
    vColor = aColor;
    vBrightness = aBrightness;  // Pass brightness to fragment shader

    // On reconstruit les coordonnées UV du quad (-1.0 → -1.0) -> (0.0, 0.0)
    vUV = aQuad * 0.5 + 0.5;            

    mat3 mat_model = build_world_matrix(size, angle);
    vec2 world_pos = (mat_model * vec3(aQuad, 1.0)).xy;

    // Clip space
    float x = world_pos.x / uSize.x * 2.0 - 1.0;
    float y = world_pos.y / uSize.y * 2.0 - 1.0;
    gl_Position = vec4(x, y, 0.0, 1.0);
}
