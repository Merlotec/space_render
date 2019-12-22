#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(std140, set = 0, binding = 0) uniform ViewArgs {
    uniform mat4 proj;
    uniform mat4 view;
    uniform mat4 proj_view;
};

layout(location = 0) in vec3 pos;

layout(location = 0) out vec2 ndc;
layout(location = 1) flat out mat4 _view;
layout(location = 5) flat out mat4 inv_proj;
layout(location = 9) flat out mat4 _proj;

void main() {
    ndc = pos.xy;
    _view = view;
    inv_proj = inverse(proj);
    _proj = proj;
    gl_Position = vec4(pos, 1.0);
}