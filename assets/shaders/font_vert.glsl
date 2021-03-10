#version 330
layout (location=0) in vec4 vertex;
out vec2 text_cords;

uniform mat4 projection;

void main() {
    text_cords = vertex.zw;
    gl_Position = projection * vec4(vertex.xy, 0.0, 1.0);
}