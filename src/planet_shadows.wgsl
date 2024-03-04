@group(0) @binding(0) var<uniform> camera: Camera;

struct Camera {
    view: mat4x4<f32>,
    view_rotation: mat3x3<f32>,
    projection: mat4x4<f32>,
    world_position: vec3<f32>,
};


struct Instance {
    position: vec3<f32>,
    shadow_direction: vec3<f32>,
    shadow_length: f32,
    shadow_height: f32,
    radius: f32,

}

let vert_position_lut: array<vec2<f32>,4> = array(
  vec2(0.0,1.0, 0.0),
  vec2(0.0,-1.0, 0.0),
  vec2(1.0,1.0, 0.0),
  vec2(1.0,-1.0, 0.0),
);

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32, in: VertexInput , instance: Instance) -> VertexOutput {
    let camera_space_origin = vec4(instance.position,1.0) * camera.view;

    // let vert_position = camera_space_origin + 
}
