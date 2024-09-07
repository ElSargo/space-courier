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
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};
struct InstanceInput {
  @location(0) position: vec3<f32>,  
  @location(1) scale: f32,
};
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32, vert: VertexInput,     instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    let view_position = camera.view * vec4(instance.position,1.0) + vec4(vert.position, 0.0, 0.0);
    out.clip_position = camera_proj * view_position;
    out.uv = vert.uv;
}
