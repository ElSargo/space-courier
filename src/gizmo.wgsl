@group(0) @binding(0) var diff: texture_2d<f32>;
@group(0) @binding(1) var diff_sampler: sampler;

@group(1) @binding(0) var<uniform> camera: Camera;

struct Camera {
    view_proj: mat4x4<f32>,
    world_position: vec3<f32>,
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

    @location(9) color: vec4<f32>,
};


struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) normal: vec3<f32>,
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

    let vert_position = model_matrix * vec4(vert.position, 1.0);
    out.clip_position =  camera.view_proj * vert_position;
    out.normal = vert.normal;
    out.color = instance.color;
    return out;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let nor = normalize(in.normal);
    let a = abs(nor);
    return in.color * max(a.z,max(a.x,a.y)) * 6.0;
}

