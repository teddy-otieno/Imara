#version 330
in vec2 text_cords;

out vec4 color;

uniform sampler2D text;
uniform vec3 text_color;

void main() {
    vec4 sampled = vec4(1.0, 1.0, 1.0, texture(text, text_cords).r);
    color = vec4(text_color, 1.0) * sampled;
    //color = vec4(1.0, 1.0, 1.0, 1.0);
}