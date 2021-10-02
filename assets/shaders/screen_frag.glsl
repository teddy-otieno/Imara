#version 330 core

in vec2 TexCoords;
out vec4 FragColor;

uniform sampler2D screen_texture;

void main() {
        //FragColor = vec4(TexCoords.x, TexCoords.y, 0.0, 1.0);
        //FragColor = vec4(TexCoords.x, TexCoords.y, 0.0, 1.0);
        FragColor = vec4(texture(screen_texture, TexCoords).rgb, 1.0);//+ (vec4(TexCoords.x, TexCoords.y, 0.0, 1.0) * 0.4);
}
