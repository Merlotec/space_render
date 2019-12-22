#version 450
#extension GL_ARB_separate_shader_objects : enable

const float PI = 3.14159265359;
const float OUTER_SCALE_FACTOR = 5;
const float DISTANCE_FACTOR = 0.0001;
layout(std140, set = 0, binding = 0) uniform ViewArgs {
    uniform mat4 proj;
    uniform mat4 view;
    uniform mat4 proj_view;
};

const uint MAX_STARS = 4;
struct StarData {
    vec3 center;
    float radius;
    vec3 color;
};

layout(std140, set = 1, binding = 0) uniform Stars {
    uint star_count;
    StarData[MAX_STARS] stars;
};

layout(location = 0) in vec3 pos;
layout(location = 1) in vec2 uv;

layout(location = 0) flat out uint idx;
layout(location = 1) out vec2 _uv;
layout(location = 2) out vec2 norm_pos;

float calc_glow_size(float r, float temp, float dist) {
    const float DSUN = 1392684.0;
    const float TSUN = 5778.0;

    float d = dist; 
    float D = (r * 2) * DSUN;
    float L = (D * D) * pow(temp / TSUN, 4.0);
    return 0.016 * pow(L, 0.25) / pow(d, 0.5);
}

void main() {
    StarData star = stars[gl_InstanceIndex];
    vec4 c_worldspace = view * vec4(star.center, 1);
    float dist = length(star.center - (-view[3].xyz));
    float dist_scale_factor =  1.0 + (dist * DISTANCE_FACTOR);
    //float glow_scale = calc_glow_size(star.radius, star.temperature, dist);
    float diagonal_factor = star.radius / cos(PI/4);
    vec3 scaled_offset = pos * diagonal_factor * dist_scale_factor * OUTER_SCALE_FACTOR;
    vec3 cameraspace = c_worldspace.xyz + scaled_offset;
    vec4 screenspace = proj * vec4(cameraspace, 1);
    idx = gl_InstanceIndex;
    norm_pos = (pos.xy + vec2(1.0)) / 2.0;
    _uv = uv;
    gl_Position = screenspace;
}