#version 330 core

in vec2 TexCoords;

out vec4 FragColor;

uniform sampler2D scene_texture;
uniform sampler2D ui_texture;

void main() {
    //FragColor = vec4(TexCoords.x, TexCoords.y, 0.0, 1.0);
    //FragColor = vec4(TexCoords.x, TexCoords.y, 0.0, 1.0);
    if(texture(ui_texture, TexCoords).xyz == vec3(0.0, 0.0, 0.0)) {
        FragColor = texture(scene_texture, TexCoords);
    } else {
        FragColor = texture(ui_texture, TexCoords);
    }
}
