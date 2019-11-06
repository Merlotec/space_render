#version 450
#extension GL_ARB_separate_shader_objects : enable

const vec2 CENTER = vec2(0.5, 0.5);

const float GLOW_START_DIST = 0.35;
const float GLOW_FACTOR = 0.5;

/// This should be a contant square.
layout(location = 0) in vec2 tex_coords;
layout(location = 1) in vec3 color;

layout(location = 0) out vec4 target;

void main() {
    float dist = distance(tex_coords, CENTER);
    float alpha = 1.0;
    if (dist > GLOW_START_DIST) {
        alpha = (((CENTER.x - GLOW_START_DIST) - dist) / GLOW_START_DIST) * GLOW_FACTOR;
    }
    target = vec4(color, alpha);
}
