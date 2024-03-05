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

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) cone_origin: vec3<f32>,
    @location(1) umbra_point: vec3<f32>,
    @location(2) penumbra_point: vec3<f32>,
    @location(3) radius: f32,
    @location(4) umbra_radius: f32,
    @location(5) penumbra_radius: f32,
};

//             3
// 1      
// O
// 2
//             4
let vert_y_displacement_lut: array<f32,4> = array( 1.0,1.0,0.0,0.0 );

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32, in: VertexInput , instance: Instance) -> VertexOutput {

    var out: VertexOutput;
    
    let camera_space_origin = vec4(instance.position,1.0) * camera.view;
    let camera_space_shadow_direction = instance.shadow_direction * camera.view_rotation;
    let lut = vert_displacement_lut[in_vertex_index];
    let y_displacement = dot(vec2(instance.radius,instance.shadow_height), vec2(lut,1.0 - lut)) ;
    let position = camera_space_origin + instance.shadow_length * camera_space_shadow_direction;
    out.clip_position = camera.projection * vec4(position,1.0);
    out.cone_origin = camera_space_origin;
    out.umbra_point = camera_space_origin + camera_space_shadow_direction * instance.shadow_length;
    out.radius = instance.radius;
    out.umbra_radius = instance.shadow_height;
    out.cone = cone;
}
