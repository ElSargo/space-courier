@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var background: texture_cube<f32>;
@group(1) @binding(1) var background_sampler: sampler;


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
    @location(0) relative_position: vec3<f32>,
    @location(1) mesh_position: vec3<f32>,
}

fn drawObject(v: vec3f) -> f32 {
    let p = abs(fract(v) - 0.5);
    return max(max(p.x, p.y), p.z);
}

const PI: f32 = 3.1415926535897932384626433832795;

fn cellTile(v: vec3f) -> f32 {
    var p = v;
    // Draw four overlapping objects (spheres, in this case) at various positions throughout the tile.
    var d: vec4f;
    d.x = drawObject(p - vec3(0.81, 0.62, 0.53));
    p.x = (p.y - p.x) * 0.7071;
    p.y = (p.y + p.x) * 0.7071;
    //p.xy = vec2(p.y*.866025 - p.x*.5, p.y*.5 + p.x*.866025); // Etc.
    d.y = drawObject(p - vec3(.39, .2, .11));
    p.y = (p.z - p.y) * 0.7071;
    p.z = (p.z + p.y) * 0.7071;
    //p.yz = vec2(p.z*.866025 - p.y*.5, p.z*.5 + p.y*.866025); // Etc.
    d.z = drawObject(p - vec3(.62, .24, .06));
    p.x = (p.z - p.x) * 0.7071;
    p.z = (p.z + p.x) * 0.7071;
    //p.xz = vec2(p.z*.866025 - p.x*.5, p.z*.5 + p.x*.866025); // Etc.
    d.w = drawObject(p - vec3(.2, .82, .64));

    // Obtain the minimum, and you're done.
    d.x = min(d.x, d.y);
    d.y = min(d.z, d.w);

    return 1. - min(d.x, d.y) * 2.; // Scale between zero and one... roughly.
    
    // For anyone wanting to experiment with this, the following gives better variance:
    //const float scale = 1.; // 1 up to 4, or higher, depending on the look you want.
    // Obviously, for the reverse, you take the one and minus away.
    //return 1. - min(min(d.x, d.y)*2.*scale, 1.);
}



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

    let vert_position = model_matrix * vec4(vert.position, 1.0)  ;
    out.relative_position = vert_position.xyz;
    out.clip_position = camera.view_proj * vert_position;
    out.mesh_position = vert.position;
    return out;
}

fn rot(a: f32) -> mat2x2<f32> {
    let s = sin(a);
    let c = cos(a);
    return mat2x2(
        c, s,
        -s, c
    );
}


fn pow2(x: vec3f) -> vec3f {
    return x * x;
}

fn fresnel(rd: vec3<f32>, normal: vec3<f32>, amount: f32) -> f32 {
    return pow((1.0 - max(dot(normal, -rd), 0.0)), amount);
}

fn exp_int(a: f32, b: f32) -> f32 {
    let e = exp(a * 2.45);
    return e / (1.0 + e) * 1.77;
}

fn smin(a: f32, b: f32, k: f32) -> f32 {
    let j = k * 4.0;
    let h = max(k - abs(a - b), 0.0) / j;
    return min(a, b) - h * h * k * (1.0 / 4.0);
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {


    let sin_60 = sin(PI / 3.0) ;
    let period = 2.0 / sqrt(3.0) ;
    let tri_coord = vec3(in.mesh_position.xz * mat2x2(sin_60, 0.5, sin_60, -0.5), in.mesh_position.z);
    let triangle = abs(fract(0.5 + tri_coord * period) - 0.5);
    let closest_tri_line = min(triangle.x, min(triangle.y, triangle.z));
    var aa = fwidth(in.mesh_position.x) + fwidth(in.mesh_position.z) ;
    // if in.clip_position.x < 900.0 {
    //     aa = 0.0;
    // }
    // let line_signal = smoothstep(0.02 + aa * 1.0,0.01, closest_tri_line) / (1.0 + aa * 100.0);
    let c = exp(-30. * aa) * 0.9 + 0.1;
    let line_signal = exp(-150.0 * closest_tri_line * c) * pow(c, 0.75);


    let rd = normalize(in.relative_position.xyz);
    let nor_flat = normalize(cross(dpdx(in.relative_position), dpdy(in.relative_position)));
    let re = reflect(rd, nor_flat);
    let bg = textureSample(background, background_sampler, re) * step(0.0, re.y);


    let fr = fresnel(rd, nor_flat, 5.0);
    let shi = pow(max(0.0, dot(re, rd)), 2.0) * fr;

    // return vec4(1.0);

    return vec4(
        vec3(1.0, 0.3, 0.05) * 0. + line_signal * vec3(1.0, 0.0, 0.23) * 20.0 * 1.0 + fr * 1.0 * shi * bg.rgb * 3.0, 
    // abs(re),
        1.0
    );
}
