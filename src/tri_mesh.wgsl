@group(0) @binding(0) var<uniform> camera: Camera;
const PI: f32 = 3.1415926535897932384626433832795;

@group(1) @binding(0) var background: texture_cube<f32>;
@group(1) @binding(1) var background_sampler: sampler;


struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,

    @location(9) rotation_only_matrix_0: vec3<f32>,
    @location(10) rotation_only_matrix_1: vec3<f32>,
    @location(11) rotation_only_matrix_2: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) distance: f32,
    @location(2) mesh_position: vec3<f32>,
    @location(3) index: f32,
}

fn drawObject(v: vec3f) -> f32 {
    let p = abs(fract(v) - 0.5);
    return max(max(p.x, p.y), p.z);
}

fn cellTile(v: vec3f) -> f32 {
    var p = v;
    // Draw four overlapping objects (spheres, in this case) at various positions throughout the tile.
    var d: vec4f;
    d.x = drawObject(p - vec3(0.81, 0.62, 0.53));
    p.x = (p.y - p.x) * 0.7071;
    p.y = (p.y + p.x) * 0.7071;
    //p.xy = vec2(p.y*.866025 - p.x*.5, p.y*.5 + p.x*.866025); // Etc.
    d.y = drawObject(p - vec3(.39, .2, .11));
    p.y = (p.z - p.y) * 0.7071;
    p.z = (p.z + p.y) * 0.7071;
    //p.yz = vec2(p.z*.866025 - p.y*.5, p.z*.5 + p.y*.866025); // Etc.
    d.z = drawObject(p - vec3(.62, .24, .06));
    p.x = (p.z - p.x) * 0.7071;
    p.z = (p.z + p.x) * 0.7071;
    //p.xz = vec2(p.z*.866025 - p.x*.5, p.z*.5 + p.x*.866025); // Etc.
    d.w = drawObject(p - vec3(.2, .82, .64));

    // Obtain the minimum, and you're done.
    d.x = min(d.x, d.y);
    d.y = min(d.z, d.w);

    return 1. - min(d.x, d.y) * 2.; // Scale between zero and one... roughly.
    
    // For anyone wanting to experiment with this, the following gives better variance:
    //const float scale = 1.; // 1 up to 4, or higher, depending on the look you want.
    // Obviously, for the reverse, you take the one and minus away.
    //return 1. - min(min(d.x, d.y)*2.*scale, 1.);
}



@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32, vert: VertexInput, instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    let rotation_matrix = mat3x3<f32>(
        instance.rotation_only_matrix_0,
        instance.rotation_only_matrix_1,
        instance.rotation_only_matrix_2,
    );

    // let disp_factor = 2.0;// (0.95+0.05*sin(vert.position*100.0));

    var position = vert.position;

    let ang = PI / 6.0 ;
    
    
    // out.index = modf(position.x).fract; 

    position.y = smoothstep(0.5,1.0,cellTile(position * 0.1)) * 2.0 + 0.0 * abs(0.4 + (sin(position.z * 0.5 + sin(position.x * 0.5))));
    // position.y = pow(position.y, 3.0) ;
    out.index = cellTile(position * 0.2) * 1.1;
    
    let vert_position = model_matrix * vec4((position - vec3(25.0, 1.0, 25.0)) * 200.0 , 1.0)  ;
    out.world_position = vert_position.xyz;
    // out.world_normal = rotation_matrix * vert.normal;
    out.distance = distance(vert_position.xyz, camera.view_pos.xyz);
    out.clip_position = camera.view_proj * vert_position;
    // out.uv = vert.uv;
    out.mesh_position = position;
    return out;

}

fn rot(a: f32) -> mat2x2<f32> {
    let s = sin(a);
    let c = cos(a);
    return mat2x2(c,s,-s,c);
}


fn fresnel(rd: vec3<f32>, normal: vec3<f32>, amount: f32) -> f32 {
    return pow((1.0 - max(dot(normal, -rd), 0.0)), amount);
}
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {


    let ang =  1. * PI / 3.0 ;
    let period = sqrt(3.0) * 0.5;
    let offset = 0.5;
    let rd = normalize(in.world_position.xyz - camera.view_pos.xyz);
    let r = rot(ang);
    
    let tri_coord = vec2((in.mesh_position.xz * rot(ang*2.0)).y, (in.mesh_position.xz * rot(ang )).y);
    let triangle_a = modf(100.5 + tri_coord.x / period  ).fract - 0.5;
    let triangle_b = modf( 100.5 + tri_coord.y / period ).fract - 0.5;
    let fake_pos = vec3(in.world_position.x, in.index * 4.0, in.world_position.z);
    let nor_flat = normalize(cross(dpdx(in.world_position ), dpdy(in.world_position )));
    // let nor_flat = normalize(cross(dpdx(fake_pos ), dpdy(fake_pos )));
    let re = reflect(rd, nor_flat);

    let bg = textureSample(background, background_sampler, re) *step(0.0,re.y);


    let light: f32 = max(
        // 0.0,
        max(
        smoothstep(0.02, 0.01, abs(triangle_a)),
        smoothstep(0.02, 0.01, abs(triangle_b)),
        ),
        // 0.0,
        smoothstep(0.02, 0.01, abs(modf(100.5 + in.mesh_position.z / period).fract - 0.5)),
        
    );
    let fr = fresnel(rd, nor_flat, 5.0);
    let shi = pow(max(0.0,dot(re,rd)), 2.0) * fr;
    
    let c = 0.0;
    return vec4(
    vec3(1.0, 0.3, 0.05) * 0. + light * vec3(1.0,0.0,0.23) * 20.0 * 1.0 + fr * 1.0 * shi * bg.rgb * 3.0, 
    // abs(re),
    1.0);
}
