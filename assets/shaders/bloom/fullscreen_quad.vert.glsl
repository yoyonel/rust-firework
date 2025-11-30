#version 330 core

// Fullscreen quad vertex shader
// Generates a fullscreen triangle without vertex buffer

out vec2 vTexCoord;

void main() {
    // Generate fullscreen triangle using vertex ID
    // Vertex 0: (-1, -1) -> TexCoord (0, 0)
    // Vertex 1: ( 3, -1) -> TexCoord (2, 0)
    // Vertex 2: (-1,  3) -> TexCoord (0, 2)
    float x = float((gl_VertexID & 1) << 2) - 1.0;
    float y = float((gl_VertexID & 2) << 1) - 1.0;
    
    vTexCoord = vec2((x + 1.0) * 0.5, (y + 1.0) * 0.5);
    gl_Position = vec4(x, y, 0.0, 1.0);
}
