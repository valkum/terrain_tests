#version 450

// Inputs
layout(triangles) in;
layout(location = 0) in vec3 in_normal[];
layout(location = 1) in vec2 in_uv[];
layout(location = 2) in vec3 in_view_vec[];
layout(location = 3) in vec3 in_world_pos[];
layout(location = 4) in float in_tesselation_level[];

// Uniforms
layout (std140, set = 0, binding = 0) uniform Projview {
    mat4 proj;
    mat4 view;
};

layout (std140, set = 1, binding = 0) uniform Args {
    mat4 model;
    vec2 terrain_size;
    float terrain_height_scale;
    float terrain_height_offset;
    bool wireframe;
};
layout(set = 1, binding = 1) uniform sampler2D terrain_height_tex;
layout(set = 1, binding = 2) uniform sampler2D normal;


// Outputs
// layout(triangle_strip, max_vertices = 4) out;
layout(line_strip, max_vertices = 6) out;
layout(location = 0) out VertexData {
    vec3 position;
    vec3 normal;
    vec2 tex_coord;
    vec4 color;
} vertex;
// layout(location = 0) out vec3 out_normal;
// layout(location = 1) out vec2 out_uv;
// layout(location = 2) out vec3 out_view_vec;
// layout(location = 3) out vec3 out_world_pos;
// layout(location = 4) out vec4 wire_color;
layout(location = 4) noperspective out vec3 edge_dist;

vec4 calc_wireframe_color()
{
    if (in_tesselation_level[0] == 64.0)
        return vec4(0.0, 0.0, 1.0, 1.0);
    else if (in_tesselation_level[0] >= 32.0)
        return vec4(0.0, 1.0, 1.0, 1.0);
    else if (in_tesselation_level[0] >= 16.0)
        return vec4(1.0, 1.0, 0.0, 1.0);
    else if (in_tesselation_level[0] >= 8.0)
        return vec4(1.0, 1.0, 1.0, 1.0);
    else
        return vec4(1.0, 0.0, 0.0, 1.0);
}


void main(void)
{
    vec4 wire_color = calc_wireframe_color();
    bool normals = true;
 
    // Calculate edge distances for wireframe
    float ha, hb, hc;
    if (wireframe)
    {
        vec2 viewport = vec2(1024, 768);
        vec2 p0 = vec2(viewport * (gl_in[0].gl_Position.xy / gl_in[0].gl_Position.w));
        vec2 p1 = vec2(viewport * (gl_in[1].gl_Position.xy / gl_in[1].gl_Position.w));
        vec2 p2 = vec2(viewport * (gl_in[2].gl_Position.xy / gl_in[2].gl_Position.w));

        float a = length(p1 - p2);
        float b = length(p2 - p0);
        float c = length(p1 - p0);
        float alpha = acos( (b*b + c*c - a*a) / (2.0*b*c) );
        float beta = acos( (a*a + c*c - b*b) / (2.0*a*c) );
        ha = abs( c * sin( beta ) );
        hb = abs( c * sin( alpha ) );
        hc = abs( b * sin( alpha ) );
    } else
    {
        ha = hb = hc = 0.0;
    }


    
        


    // Output verts
    for(int i = 0; i < gl_in.length(); ++i)
    {
        gl_Position = proj * view * gl_in[i].gl_Position;
        vertex.position = gl_in[i].gl_Position.xyz;
        // out_uv = in_uv[i];
        vertex.tex_coord = in_uv[i];
        vertex.normal = in_normal[i];
        // out_view_vec = in_view_vec[i];
        // out_world_pos = in_world_pos[i];
        // wire_color = wire_color;
        vertex.color = wire_color;

        if (i == 0)
            edge_dist = vec3(ha, 0, 0);
        else if (i == 1)
            edge_dist = vec3(0, hb, 0);
        else
            edge_dist = vec3(0, 0, hc);

        EmitVertex();

    }

    // This closes the the triangle
    gl_Position = proj * view * gl_in[0].gl_Position;
    vertex.position = gl_in[0].gl_Position.xyz;
    edge_dist = vec3(ha, 0, 0);
    // out_uv = in_uv[0];
    vertex.tex_coord = in_uv[0];
    vertex.normal = in_normal[0];
    // out_view_vec = in_view_vec[0];
    // out_world_pos = in_world_pos[0];
    // gs.wire_color = wire_color;
    vertex.color = wire_color;
    EmitVertex();
    EndPrimitive();

    if(normals) {
        for(int i = 0; i < gl_in.length(); ++i) {
            vec3 normal = texture(normal, in_uv[i]).rgb;
            normal = normal * 2. - 1.;

            vec3 vertex_normal = normalize(in_normal[i]);
            vec3 vertex_tangent = normalize(vec3(0.0, 0.0, 1.0));
            vec3 vertex_bitangent = normalize(cross(vertex_normal, vertex_tangent));
            mat3 vertex_basis = mat3(vertex_tangent, vertex_bitangent, vertex_normal);
            normal = normalize(vertex_basis * normal);

            vec3 P = gl_in[i].gl_Position.xyz;
            vec3 N = normal;

            gl_Position = proj * view * gl_in[i].gl_Position;
            vertex.position = gl_in[i].gl_Position.xyz;
            vertex.tex_coord = in_uv[i];
            vertex.normal = normal;
            vertex.color = vec4(1.0, 0.0, 0.0, 1.0);
            EmitVertex();
            
            gl_Position = proj * view * vec4(gl_in[i].gl_Position.xyz + N * 0.25, 1.0);
            vertex.position = gl_Position.xyz;
            vertex.tex_coord = in_uv[i];
            vertex.normal = normal;
            vertex.color = vec4(1.0, 0.0, 0.0, 1.0);
            EmitVertex();
            EndPrimitive();
        }
    }

    
}