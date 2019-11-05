#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(std140, set = 0, binding = 0) uniform ViewArgs {
    uniform mat4 proj;
    uniform mat4 view;
    uniform mat4 proj_view;
};

struct StarData {
    vec2 spherical_coords;
    vec3 color;
    float scale;
};

layout(std140, set = 1, binding = 0) buffer StarList {
    StarData stars[];
};

layout(location = 0) in vec3 position;
layout(location = 1) in vec2 tex_coord;

layout(location = 0) out vec2 tex_coord_out;
layout(location = 1) out vec3 color_out;

const float PI = 3.1415926535897932384626433832795;

vec4 quat_from_axis_angle(vec3 axis, float angle)
{
    vec4 qr;
    float half_angle = (angle * 0.5);
    qr.x = axis.x * sin(half_angle);
    qr.y = axis.y * sin(half_angle);
    qr.z = axis.z * sin(half_angle);
    qr.w = cos(half_angle);
    return qr;
}

vec4 quat_conj(vec4 q)
{
    return vec4(-q.x, -q.y, -q.z, q.w);
}

vec4 quat_mult(vec4 q1, vec4 q2)
{
    vec4 qr;
    qr.x = (q1.w * q2.x) + (q1.x * q2.w) + (q1.y * q2.z) - (q1.z * q2.y);
    qr.y = (q1.w * q2.y) - (q1.x * q2.z) + (q1.y * q2.w) + (q1.z * q2.x);
    qr.z = (q1.w * q2.z) + (q1.x * q2.y) - (q1.y * q2.x) + (q1.z * q2.w);
    qr.w = (q1.w * q2.w) - (q1.x * q2.x) - (q1.y * q2.y) - (q1.z * q2.z);
    return qr;
}

vec3 rotate_vertex_position(vec3 pos, vec4 qr)
{
    vec4 qr_conj = quat_conj(qr);
    vec4 q_pos = vec4(pos.xyz, 0);

    vec4 q_tmp = quat_mult(qr, q_pos);
    qr = quat_mult(q_tmp, qr_conj);

    return qr.xyz;
}

vec3 calc_axis_xz(float radians) {
    float angle = radians;
    return normalize(vec3(cos(angle), 0, sin(angle)));
}

void main() {
    // Fetch the star data for this particular instance.
    StarData star_data = stars[gl_InstanceIndex];

    // Apply scale (only to x and y components, applying to z would push it further back, causing it to stauy the same size due to perspective).
    vec3 scaled = vec3(position.xy * star_data.scale, position.z);

    // Calculating rotations.
    vec4 q_y = quat_from_axis_angle(vec3(0.0, 1.0, 0.0), star_data.spherical_coords.x);
    // The vec3 used is the x axis.
    vec4 q_x = quat_from_axis_angle(vec3(1.0, 0.0, 0.0), star_data.spherical_coords.y);
    vec4 qr = quat_mult(q_y, q_x);

    vec3 rotated = rotate_vertex_position(scaled, qr);

    // Calculate the view matrix without the translation, since the background should not move which the camera (as it is infinately far away).
    mat4 view_without_translation = view;
    view_without_translation[3].xyz = vec3(0.0f, 0.0f, 0.0f);

    vec4 screenspace = (proj * view_without_translation * vec4(rotated, 1.0));

    // Send to fragment shader.
    tex_coord_out = tex_coord;
    color_out = star_data.color;
    gl_Position = screenspace.xyww;
}
