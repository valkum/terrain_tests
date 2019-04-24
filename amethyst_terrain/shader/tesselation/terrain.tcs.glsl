#version 450


// Inputs
layout(location = 0) in vec3 in_normal[];
layout(location = 1) in vec2 in_uv[];
layout(location = 2) in ivec4 in_neighbour_scales[];

// Uniforms
layout (std140, set = 0, binding = 0) uniform Args {
    mat4 proj;
    mat4 view;
    mat4 model;
    vec2 terrain_size;
    float terrain_height_scale;
    float terrain_height_offset;
    float wireframe;
};

layout (set = 1, binding = 0) uniform sampler2D terrain_height_tex;



// Outputs
layout(vertices = 4) out;

layout(location = 0) out vec3 out_normal[4];
layout(location = 4) out vec2 out_uv[4];
layout(location = 8) out float out_tesselation_level[4];



void main()
{
    gl_TessLevelOuter[0] = max(2.0, in_neighbour_scales[gl_InvocationID].w);
    gl_TessLevelOuter[1] = max(2.0, in_neighbour_scales[gl_InvocationID].x);
    gl_TessLevelOuter[2] = max(2.0, in_neighbour_scales[gl_InvocationID].y);
    gl_TessLevelOuter[3] = max(2.0, in_neighbour_scales[gl_InvocationID].z);


    // Inner tessellation level
    gl_TessLevelInner[0] = 0.5 * (gl_TessLevelOuter[0] + gl_TessLevelOuter[3]);
    gl_TessLevelInner[1] = 0.5 * (gl_TessLevelOuter[2] + gl_TessLevelOuter[1]);

    // Pass the patch verts along
    gl_out[gl_InvocationID].gl_Position = gl_in[gl_InvocationID].gl_Position;

    out_normal[gl_InvocationID] = in_normal[gl_InvocationID];

    // Output heightmap coordinates
    out_uv[gl_InvocationID] = in_uv[gl_InvocationID];

    // Output tessellation level (used for wireframe coloring)
    // tcs[gl_InvocationID].tesselation_level = gl_TessLevelOuter[0];
    out_tesselation_level[gl_InvocationID] = 0.5 * (gl_TessLevelInner[0] + gl_TessLevelInner[1]);
}