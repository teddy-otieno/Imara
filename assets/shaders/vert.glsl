#version 330 core
layout (location = 0) in vec3 position;
layout (location = 1) in vec3 normal;

uniform mat4 view;
uniform mat4 pers;
uniform mat4 model;

out vec3 frag_norm;
out vec3 frag_position;

void main() {
    
    frag_norm = mat3(transpose(inverse(model))) * normal;
    frag_position = position;

    gl_Position = (pers * view * model) * vec4(position, 1.0);
}
