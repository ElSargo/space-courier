@group(0) @binding(0) var<uniform> camera: Camera;
struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3f,
};

struct InstanceInput {
    @location(1) model_matrix_0: vec4<f32>,
    @location(2) model_matrix_1: vec4<f32>,
    @location(3) model_matrix_2: vec4<f32>,
    @location(4) model_matrix_3: vec4<f32>,
    
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32, vert: VertexInput, instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );

    var color = vec3(0.0);
    let i = in_vertex_index % 4;
    if i == 1 {
        color.r = 1.0;
    }
    if i % 4 == 2 {
        color.g = 1.0;
    }
    if i % 4 == 3 {
        color.b = 1.0;
    }

    let relative_position = model_matrix * vec4(vert.position, 1.0);

    var out: VertexOutput;

    out.clip_position = camera.view_proj * relative_position;
    out.color = color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4(4.0 * vec3(0.6, 0.45, 2.0), 1.0);
}
