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
    @location(0) position: vec4<f32>,
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
    let vert_position = vec4(vert.position.xz + instance.translation.xy, vert.position.y, 1.0).xzyw  ;
    return VertexOutput(
        camera.view_proj * vert_position,
        vert_position.xyz - vec3(0.0, camera.view_pos.y, 0.0),
        vert.position.xyz
    );
}



fn fresnel(ray_direction: vec3<f32>, normal: vec3<f32>, ammount: f32) -> f32 {
    return pow((1.0 - max(dot(normal, -ray_direction), 0.0)), ammount);
}

fn step_intergral_indefinite(coordinates: vec3f, line_width: f32) -> vec3f {
    let period = 3.0 / sqrt(12.0);
    let periodic_coordinates = fract(coordinates / period) * period;
    let floored_component = line_width / period * (coordinates - periodic_coordinates);
    let fractional_component = clamp(periodic_coordinates - (period + line_width) / 2.0, vec3(0.0), vec3(line_width));
    return floored_component + fractional_component;
}

fn triangle_line_signal(coordinates: vec3f, line_width: f32) -> vec3f {
    let delta_coordinates = fwidth(coordinates);
    let upper = step_intergral_indefinite(coordinates + delta_coordinates + line_width, line_width);
    let lower = step_intergral_indefinite(coordinates - delta_coordinates + line_width, line_width);
    return (upper - lower) / (2.0 * delta_coordinates);
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

    let tri_int = triangle_line_signal(tri_coord, 0.02) ;
    let line_signal = max(tri_int.x, max(tri_int.y, tri_int.z));


    let ray_direction = normalize(in.relative_position.xyz);
    let flat_normal = normalize(cross(dpdx(in.relative_position), dpdy(in.relative_position)));
    let reflected_ray_direction = reflect(ray_direction, flat_normal);

    let planer_reflection_incident = sample_planar_reflection(in.clip_position.xy) * step(0.99, flat_normal.y);
    let background_incident = textureSample(background, background_sampler, reflected_ray_direction).rgb;

    let reflection_incident = mix(background_incident.rgb, planer_reflection_incident.rgb, planer_reflection_incident.a);

    let fresnel_strength = fresnel(ray_direction, flat_normal, 5.0);
    let reflection_strength = pow(max(0.0, dot(reflected_ray_direction, ray_direction)), 2.0) * fresnel_strength + 0.2;

    let line_color = vec3(1.0, 0.0, 0.23) * 20.0 * (1.0 - exp(-in.mesh_position.y) + 0.03);
    let reflection_color = reflection_strength * reflection_incident;
    // let color = mix(reflection_color, line_color, line_signal);
    let color = reflection_color + line_color * line_signal + vec3(0.4, 0.3, 2.0) * 0.001  ;

    return vec4(color, 1.0);
}
