@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var diff: texture_2d<f32>;
@group(1) @binding(1) var diff_sampler: sampler;


@group(2) @binding(0) var background: texture_cube<f32>;

// @group(2) @binding(0) var<uniform> model: Model;

// struct Model {
//     matrix: mat4x4<f32>,
// };

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) normal: vec3<f32>,
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
    @location(0) uv: vec2<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) distance: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32, vert: VertexInput,     instance: InstanceInput,
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
    
    let vert_position = model_matrix * vec4(vert.position, 1.0);
    out.world_position = vert_position.xyz;
    out.world_normal =  rotation_matrix * vert.normal;
    out.distance = distance(vert_position.xyz, camera.view_pos.xyz);
    out.clip_position =  camera.view_proj * vert_position;
    out.uv = vert.uv;
    return out;
}

// Fragment shader
fn fresnel(rd: vec3<f32>,  normal: vec3<f32>,amount: f32) -> f32 {
	return pow((1.0 - max(dot(normal, -rd), 0.0)), amount);
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let rd = normalize(in.world_position - camera.view_pos.xyz);
    let nor = normalize(in.world_normal);
    let dis = length(in.world_position * 0.005);
    let sun = normalize(-in.world_position);
    let dif = max(0.0, dot(nor, sun));
    let frez = fresnel(rd,nor,5.0);
    let refl = reflect(rd,nor);
    let spe = pow(max(0.0,dot(sun,refl)), 9.0);
    var col = textureSample(diff, diff_sampler, in.uv).rgb;
    var ems = vec3(0.0);
    if max(abs(col.r - 0.156862), max(
        abs(col.g - 0.85883),
        abs(col.b - 1.0)
    )) < 0.22 {
        ems += col * 20.0;
    }
    // let env = textureSample(background,diff_sampler, refl ).rgb * col;
    let difs =  max(0.0, dif + 0.01) * 0.5 / ( dis * dis );
    let scl = vec3(1.4,1.,1.);
    let spec = (frez*0.2 + 1.0 ) * spe  ;
    return vec4<f32>(  col * difs + vec3(0.1,0.2,0.15)*(0.0 + frez )*0.2+ ems + spec*dif, 1.0)  ;
    // return vec4(0.00001);
    // return vec4(spe*(1.+frez));
    // return vec4(spe);
    // let pos = in.world_position % vec3(1.0) ;
    // return vec4(pos,1.0);
}
