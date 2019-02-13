#version 150 core


layout (std140) uniform VertexArgs {
    uniform mat4 proj;
    uniform mat4 view;
    uniform mat4 model;
    uniform vec4 color;
};

uniform int size;
uniform float z_scale_factor;
uniform float z_tex_scale_factor;
uniform vec2 alpha_offset;
uniform vec2 one_over_width;
uniform vec3 camera_position;
uniform vec4 fine_block_orig;
uniform vec4 scale_factor;
uniform vec4 color_overwrite;

uniform sampler2D elevation_sampler;

in vec3 position;
in vec2 tex_coord;

out VertexData {
    vec3 position;
    vec2 tex_coord; // coordinates for normal-map lookup
    float z; // coordinates for elevation-map lookup
    float alpha; // transition blend
    vec2 test;
    vec4 zf_zd;
} vertex;

// Vertex shader for rendering the geometry clipmap
void main() {
    // Paper suggests to use (fmod(gl_VertexID, size), floor(gl_VertexID/size)) and avoid the vertex buffer.
    // For now we generate a block mesh with values of [-(m-1)/2; (m-1)/2]
    vec2 grid_pos = position.xy; 
    // convert from grid xy to world xy coordinates
    // Scale_factor.xy: grid spacing of current level
    // Scale_factor.zw: origin of current block within world relative ect to the center.
    vec2 world_pos = (grid_pos + scale_factor.zw) * scale_factor.xy;
    // compute coordinates for vertex texture
    // Fine_block_orig.xy: 1/(w, h) of texture
    // Fine_block_orig.zw: origin of block in texture
    vec2 uv = (grid_pos + fine_block_orig.zw) * fine_block_orig.xy;

    // sample the vertex texture
    vec4 zf_zd = textureLod(elevation_sampler, uv, 0);
    // unpack to obtain zf and zd = (zc - zf)
    // zf is elevation value in current (fine) level
    // zc is elevation value in coarser level
    float zf = zf_zd.x;
    // float zd = fract(zf_zd.x) * 512 - 256; // (zd = zc - zf)
    float zd = zf_zd.y;

    // compute alpha (transition parameter) and blend elevation
    // vec2 alpha = clamp((abs(world_pos - camera_position.xy) - alpha_offset) * one_over_width, 0, 1);
    // Use grid_pos here to avoid dealing with different alpha_offsets for each level
    vec2 alpha = clamp((abs(grid_pos + scale_factor.zw) - alpha_offset) * one_over_width, 0, 1);
    alpha.x = max(alpha.x, alpha.y); 
    float z = zf + alpha.x * zd;
    z = z * z_scale_factor;

    vec4 vertex_position = model * vec4(world_pos.x, world_pos.y, z,  1.0);
    
    vertex.position = vertex_position.xyz;
    vertex.tex_coord = uv;
    vertex.z = z * z_tex_scale_factor; 
    vertex.alpha = alpha.x;
    vertex.test = grid_pos + fine_block_orig.zw;
    vertex.zf_zd = zf_zd;
    gl_Position = proj * view * vertex_position;
}