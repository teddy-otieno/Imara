#version 330
out vec4 color;

uniform vec3 quad_color;

void main() {
    color = vec4(quad_color, 1.0);
}