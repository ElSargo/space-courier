@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var background: texture_cube<f32>;
@group(1) @binding(1) var background_sampler: sampler;

@group(2) @binding(0) var color_texture: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var depth_texture: texture_depth_2d;
@group(2) @binding(3) var depth_sampler: sampler;

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct InstanceInput {
    @location(1) translation: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) relative_position: vec3<f32>,
    @location(1) mesh_position: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32, vert: VertexInput, instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;

    let vert_position = vec4(vert.position.xz + instance.translation.xy, vert.position.y, 1.0).xzyw  ;
    out.relative_position = vert_position.xyz - vec3(0.0, camera.view_pos.y, 0.0);
    out.clip_position = camera.view_proj * vert_position;
    out.mesh_position = vert.position;
    return out;
}



fn fresnel(rd: vec3<f32>, normal: vec3<f32>, amount: f32) -> f32 {
    return pow((1.0 - max(dot(normal, -rd), 0.0)), amount);
}

fn step_int_indef(x: vec3f, i: f32) -> vec3f {
    let k = 3.0 / sqrt(12.0);
    let m = fract(x / k) * k;
    return 2.0 * i / k * (x - m) + clamp(m - k / 2.0 + i, vec3(0.0), vec3(2.0 * i));
}

fn step_int_avg(x: vec3f, i: f32) -> vec3f {
    let dx = fwidth(x);
    let acc = step_int_indef(x + dx, i) - step_int_indef(x - dx, i);
    return acc / (2.0 * dx);
}

fn sample_planar_reflection(clip_coords: vec2f) -> vec4f {
    let integer_coords = vec2<u32>(clip_coords);
    let sample_coords = vec2(textureDimensions(color_texture).x - integer_coords.x, integer_coords.y);
    return textureLoad(color_texture, sample_coords, 0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sin_60 = sqrt(3.0) / 2.0;
    let tri_coord = vec3(in.mesh_position.xz * mat2x2(sin_60, 0.5, sin_60, -0.5), in.mesh_position.z) + 3.0 / sqrt(48.0);

    let tri_int = step_int_avg(tri_coord, 0.02) ;
    let line_signal = max(tri_int.x, max(tri_int.y, tri_int.z)) * (1.0 - exp(-in.mesh_position.y) + 0.01);


    let rd = normalize(in.relative_position.xyz);
    let nor_flat = normalize(cross(dpdx(in.relative_position), dpdy(in.relative_position)));
    let re = reflect(rd, nor_flat);

    let planer_reflection_incident = sample_planar_reflection(in.clip_position.xy) * step(0.99, nor_flat.y);
    let background_incident = textureSample(background, background_sampler, re).rgb;

    let reflection_incident = mix(background_incident.rgb, planer_reflection_incident.rgb, planer_reflection_incident.a);

    let fresnel_strength = fresnel(rd, nor_flat, 5.0);
    let reflection_strength = pow(max(0.0, dot(re, rd)), 2.0) * fresnel_strength;

    let line_color = line_signal * vec3(1.0, 0.0, 0.23) * 20.0;
    let reflection_color = reflection_strength * reflection_incident;
    let color = line_color + reflection_color;

    return vec4(color, 1.0);
}
