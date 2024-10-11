@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var background: texture_cube<f32>;
@group(1) @binding(1) var background_sampler: sampler;

@group(2) @binding(0) var color_texture: texture_2d<f32>;
@group(2) @binding(1) var color_sampler: sampler;
@group(2) @binding(2) var depth_texture: texture_depth_2d;
@group(2) @binding(3) var depth_sampler: sampler;

@group(3) @binding(0) var diff: texture_2d<f32>;
@group(3) @binding(1) var diff_sampler: sampler;


// @group(2) @binding(0) var<uniform> model: Model;


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
    @builtin(vertex_index) in_vertex_index: u32, vert: VertexInput, instance: InstanceInput,
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
    out.world_normal = rotation_matrix * vert.normal;
    out.distance = distance(vert_position.xyz, camera.view_pos.xyz);
    out.clip_position = camera.view_proj * vert_position;
    out.uv = vert.uv;
    return out;
}


fn blend(a: vec3f, b: vec3f, t: f32) -> vec3f {
    let delta = a - b;
    let maxDelta = max(
        max(
            abs(delta.x),
            abs(delta.y)
        ),
        abs(delta.z)
    );
    let s = sign(delta);
    let c = a - s * maxDelta * t;

    let minc = vec3(
        min(a.x, b.x),
        min(a.y, b.y),
        min(a.z, b.z)
    );

    let maxc = vec3(
        max(a.x, b.x),
        max(a.y, b.y),
        max(a.z, b.z)
    );

    return clamp(c, minc, maxc);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let h = in.world_position.y;
    // let color = blend(vec3(1.0,0.0,0.3), vec3(1.0,0.0,1.0),-0.5+h*0.5);
    let color = vec3(0.2, 0.4, 1.0);
    let threshold = -0.1 - 0.05 * h;
    // let alpha = smoothstep(threshold,threshold + 2.1, 0.2 +  sin(h*1.));
    let alpha = 1.0;
    return vec4(color * alpha * 240.0, alpha);
}

