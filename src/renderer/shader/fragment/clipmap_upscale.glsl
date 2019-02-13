in vec2 ip_uv;


uniform vec2 texture_offset;
uniform float size;
uniform float one_over_size;
uniform vec2 viewport;

uniform sampler2D coarse_level_elevation_sampler;
uniform sampler2D residual_sampler;
uniform sampler2D lookup_sampler;

out vec4 out_color;

void main()
{
    float residual = texture(residual_sampler, ip_uv*one_over_size).r;  
    
    vec2 p_uv = floor(ip_uv);
    vec2 p_uv_div2 = p_uv/2.;
    vec2 lookup_tij = p_uv_div2+1.; 
    vec4 mask_type = texture(lookup_sampler, lookup_tij);     
          
    mat4 mask_matrix[4];
    mask_matrix[0] = mat4(0, 0, 0, 0,
                           0, -1.0f/16.0f, 0, 0,
                           0, 0, 0, 0,
                           1.0f/256.0f, -9.0f/256.0f, -9.0f/256.0f, 1.0f/256.0f);
                           
    mask_matrix[1] = mat4(0, 1, 0, 0,
                           0, 9.0f/16.0f, 0, 0,
                           -1.0f/16.0f, 9.0f/16.0f, 9.0f/16.0f, -1.0f/16.0f,
                           -9.0f/256.0f, 81.0f/256.0f, 81.0f/256.0f, -9.0f/256.0f);                        
                           
    mask_matrix[2] = mat4(0, 0, 0, 0,
                           0, 9.0f/16.0f, 0, 0,
                           0, 0, 0, 0,
                           -9.0f/256.0f, 81.0f/256.0f, 81.0f/256.0f, -9.0f/256.0f);
                           
    mask_matrix[3] = mat4(0, 0, 0, 0,
                           0, -1.0f/16.0f, 0, 0,
                           0, 0, 0, 0,
                           1.0f/256.0f, -9.0f/256.0f, -9.0f/256.0f, 1.0f/256.0f);

    vec2 offset = vec2(dot(mask_type.bgra, vec4(1, 1.5, 1, 1.5)), dot(mask_type.bgra, vec4(1, 1, 1.5, 1.5)));
    
    float z_predicted=0.;
    offset = (p_uv_div2-offset+0.5)*one_over_size+texture_offset;
    for(int i = 0; i < 4; i++) {
        float zrowv[4];
        for (int j = 0; j < 4; j++) {
                vec2 vij    = offset+vec2(i,j)*one_over_size;
                zrowv[j]      = texture(coarse_level_elevation_sampler, vij).r;
        }
        
        vec4 mask = mask_type.bgra * mask_matrix[i];
        vec4 zrow = vec4(zrowv[0], zrowv[1], zrowv[2], zrowv[3]);
        zrow = floor(zrow);
        z_predicted = z_predicted+dot(zrow, mask);
    }

    
    z_predicted = floor(z_predicted);
    
    // add the residual to get the actual elevation
    float zf = z_predicted + residual;  
    
    // zf should always be an integer, since it gets packed
    //  into the integer component of the floating-point texture
    zf = floor(zf);
    
    vec4 uvc = floor(vec4((p_uv_div2+vec2(0.5,0)), (p_uv_div2+vec2(0,0.5))))*one_over_size+texture_offset.xyxy; 
            
    // look up the z_predicted value in the coarser levels  
    // TODO: Maybe use textureLod here
    float zc0 = floor(texture(coarse_level_elevation_sampler, uvc.xy)).r;
    float zc1 = floor(texture(coarse_level_elevation_sampler, uvc.zw)).r;        
    
    float zf_zd = zf + ((zc0+zc1)/2.-zf+256.)/512.;

    out_color = vec4(zf_zd, 0, 0, 0);
}