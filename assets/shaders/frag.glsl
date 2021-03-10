#version 330 core

in vec3 frag_position;
in vec3 frag_norm;

struct DirectionalLight {
    vec3 color;
    vec3 direction;
};

uniform DirectionalLight dir_light;
uniform vec3 color;

vec3 calculate_dir_light(DirectionalLight light, vec3 normal) {
    vec3 light_dir = normalize(light.direction);
    float diff = max(dot(frag_norm, light_dir), 0.0);
    vec3 diffuse = diff * light.color;

    return diffuse;
}

void main() {
    //vec3 object_color = vec3(0.7, 0.7, 0.7);

    float ambient_strength = 0.4;
    vec3 ambient = ambient_strength * color;

    vec3 dir_light = calculate_dir_light(dir_light, frag_norm);
    vec3 result = ( dir_light + ambient) * color;
    gl_FragColor = vec4(result, 1.0);
}
