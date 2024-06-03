@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var diff: texture_2d<f32>;
@group(1) @binding(1) var diff_sampler: sampler;


// @group(2) @binding(0) var<uniform> model: Model;


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
    // var col = vec3(0.0);
    // if in.world_position.x > 0.0 {
        // col.r += 1.0;
        // return vec4(0.4,1.5,0.5,1.0)*14.0;
    // }
    // if in.world_position.y > 0.0 {
        // col.g += 1.0;
        return vec4(1.4,0.75,0.5,1.0)*14.0;
    // }
    // if in.world_position.z > 0.0 {
        // col.b += 1.0;
    //     // return vec4(1.4,0.75,0.5,1.0)*14.0;
    // }
    // return vec4(0.5,0.75,1.4,1.0)*10.0;
    // return vec4(col,1.0)*1.0;
}

