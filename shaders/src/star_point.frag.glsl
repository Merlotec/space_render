#version 450
#extension GL_ARB_separate_shader_objects : enable

const vec2 center = vec2(0.5, 0.5);

/// This should be a contant square.
layout(location = 0) in vec2 tex_coords;
layout(location = 1) in vec3 color;

layout(location = 0) out vec4 target;

void main() {
    float dist = distance(tex_coords, center);
    float alpha = (0.5 - dist) / 0.3;
    target = vec4(color, alpha);
}
