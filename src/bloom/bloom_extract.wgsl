@group(0) @binding(0) var color_sampler: sampler;
@group(0) @binding(1) var<uniform> threshold: f32;
@group(0) @binding(2) var background: texture_cube<f32>;
@group(0) @binding(3) var color_texture: texture_2d<f32>;
@group(0) @binding(4) var<uniform> camera: Camera;

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let view_pos_homogeneous = camera.inv_proj * vec4((vec2(in.uv.x, 1.0 - in.uv.y) - 0.5) * 2.0, 1.0, 1.0);
    let view_ray_direction = view_pos_homogeneous.xyz / view_pos_homogeneous.w;
    var ray_direction = normalize((camera.inv_view * vec4(view_ray_direction, 0.0)).xyz);

    let view_sample = textureSample(color_texture, color_sampler, in.uv);
    let bg_sample = textureSample(background, color_sampler, ray_direction);

    return vec4(
        max(vec3(0.0), view_sample.rgb - vec3(threshold)) + max(vec3(0.0), bg_sample.rgb - vec3(threshold)) * ( 1.0 - view_sample.a), 1.0
    );
    // return vec4(view_sample.a * 00.6);
}


