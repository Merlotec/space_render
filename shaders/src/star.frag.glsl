#version 450
#extension GL_ARB_separate_shader_objects : enable

const float PI = 3.14159265359;
const uint MAX_STARS = 4;
const float GLOW_FACTOR = 0.2;
const float OPAQUE_MARGIN_FACTOR = 1.7;
const float RAD = 0.5;
struct StarData {
    vec3 center;
    float radius;
    vec3 color;
};

layout(std140, set = 1, binding = 0) uniform Stars {
    uint star_count;
    StarData[MAX_STARS] stars;
};


layout(set = 2, binding = 0) uniform sampler2D glow_tex;

layout(location = 0) flat in uint idx;
layout(location = 1) in vec2 uv;
layout(location = 2) in vec2 norm_pos;

layout(location = 0) out vec4 target;

void main() {
    float dist = length(norm_pos - vec2(RAD, RAD));
    if (dist > RAD) {
        discard;
    }
    float base_factor = 1.0 - (dist / RAD);
    float ni_factor = pow(base_factor, 3);
    float margin_factor = ni_factor * OPAQUE_MARGIN_FACTOR;
    vec4 tex_c = texture(glow_tex, uv);
    float tex_factor = tex_c.x;
    float glow = ni_factor * GLOW_FACTOR;
    float alpha = (tex_factor + glow) * margin_factor;
    target = vec4(stars[idx].color * tex_factor, alpha);
}
