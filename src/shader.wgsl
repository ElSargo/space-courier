@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var diff: texture_2d<f32>;
@group(1) @binding(1) var diff_sampler: sampler;


@group(2) @binding(0) var background: texture_cube<f32>;

// @group(2) @binding(0) var<uniform> model: Model;

// struct Model {
//     matrix: mat4x4<f32>,
// };

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
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
    @location(4) mesh_position: vec3<f32>,
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

    let disp_factor = 1.0;// (0.95+0.05*sin(vert.position*100.0));
    let vert_position = model_matrix * vec4(vert.position * disp_factor, 1.0)  ;
    out.world_position = vert_position.xyz;
    out.world_normal = rotation_matrix * vert.normal;
    out.distance = distance(vert_position.xyz, camera.view_pos.xyz);
    out.clip_position = camera.view_proj * vert_position;
    out.uv = vert.uv;
    out.mesh_position = vert.position;
    return out;
}

// Fragment shader
fn fresnel(rd: vec3<f32>, normal: vec3<f32>, amount: f32) -> f32 {
    return pow((1.0 - max(dot(normal, -rd), 0.0)), amount);
}

fn hash33(p: vec3<f32>) -> vec3<f32> {
    var q: vec3<u32> = vec3<u32>(vec3<i32>(p)) * vec3<u32>(1597334673u, 3812015801u, 2798796415u);
    q = (q.x ^ q.y ^ q.z) * vec3<u32>(1597334673u, 3812015801u, 2798796415u);
    return vec3<f32>(q) * 2.328306437080797e-10;
}

fn dot2(x: vec3f) -> f32 {
    return dot(x, x);
}


fn cell(p: vec3f) -> f32 {


    let origin = round(p);
    var dist = 10e10;
    for (var x = 0.0; x < 1.5; x += 1.0) {
        for (var y = 0.0; y < 1.5; y += 1.0) {
            for (var z = 0.0; z < 1.5; z += 1.0) {
                let disp = vec3(x, y, z);
                let o = origin + disp;
                let dl = hash33(o) * - 0.5;
                let dif = o + dl - p;
                let nd = dot2(dif);
                dist = min(dist, nd);
            }
        }
    }
    dist = sqrt(dist);
    return dist;
}

fn cell_mat(p: vec3f) -> f32 {

    let origin = round(p);
    var dist = 10e10;

    for (var x = 0.0; x < 1.5; x += 1.0) {
        for (var y = 0.0; y < 1.5; y += 1.0) {
            for (var z = 0.0; z < 1.5; z += 1.0) {
                let disp = vec3(x, y, z);
                let o = origin + disp;
                let dl = hash33(o) * - 0.5;
                // let nd = dot2(o + dl -  p);
                let dif = abs(o + dl - p);
                let nd = dif.x + dif.y + dif.z;
                dist = min(dist, nd);
            }
        }
    }
    return dist;
}

fn cell_s(p: vec3f) -> f32 {


    let origin = round(p);
    var dist = 4.0;
    for (var x = 0.0; x < 1.5; x += 1.0) {
        for (var y = 0.0; y < 1.5; y += 1.0) {
            for (var z = 0.0; z < 1.5; z += 1.0) {
                let disp = vec3(x, y, z);
                let o = origin + disp;
                let dl = hash33(o) * - 0.5;
                let dif = o + dl - p;
                let nd = dot2((dif));

                let sm = 0.3;
        		let h = smoothstep( -1.0, 1.0, (dist-nd)/sm );
        	    dist = mix( dist, nd, h ) - h*(1.0-h)*sm/(1.0+3.0*sm);
            }
        }
    }
    // dist = exp(-dist * 3.0);
    return  dist;
}

fn cell_m_s(p: vec3f) -> f32 {


    let origin = round(p);
    var dist = 4.0;
    for (var x = 0.0; x < 1.5; x += 1.0) {
        for (var y = 0.0; y < 1.5; y += 1.0) {
            for (var z = 0.0; z < 1.5; z += 1.0) {
                let disp = vec3(x, y, z);
                let o = origin + disp;
                let dl = hash33(o) * - 0.5;
                let dif = abs(o + dl - p);
                let nd = (dif.x + dif.y + dif.z);

                let sm = 0.5;
        		let h = smoothstep( -1.0, 1.0, (dist-nd)/sm );
        	    dist   = mix( dist, nd, h ) - h*(1.0-h)*sm/(1.0+3.0*sm);
            }
        }
    }
    // dist = exp(-dist * 3.0);
    return dist;
}


// https://iquilezles.org/articles/dontflip/
fn reflVec(v: vec3f, r: vec3f) -> vec3f {
    let k = dot(v, r);
    if k > 0.0 {return v;} else {return v - 2.0 * r * k;};
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos_realative = in.world_position - camera.view_pos.xyz;
    let rd = normalize(pos_realative);
    var col = textureSample(diff, diff_sampler, in.uv).rgb;
    let water_dist = length(col - vec3(0.13725, 0.61961, 1.0));
    let nor_vert = normalize(in.world_normal);
    let nor_flat = normalize(cross(dpdx(in.world_position), dpdy(in.world_position)));
    let blend_factor = smoothstep(1.0, 0.92, water_dist);
    var cell =  1.0;
    var cxv = 0.0;
    cxv = cell_s(in.mesh_position * 4.0);
    cell += mix(1.0 - cxv,cxv,  blend_factor) ;
    cxv = cell_s(in.mesh_position * 8.0);
    cell += mix(1.0 - cxv,cxv,  blend_factor) ;
    // cell +=  cell_s(in.mesh_position * 16.0)*0.25;
    let sca_cell = 0.95 + 0.05 * cell;
    let nor_cell = normalize(cross(dpdx(sca_cell * nor_vert), dpdy(sca_cell * nor_vert)));
    // let cell2 = cell_mat_s(in.mesh_position * 10.0);
    // cell.grad = normalize(mix(cell.grad, cell2.grad, 0.3));
    // cell.value = mix(cell.value, cell2.value, 0.3);

    // let nor = normalize(nor_vert + reflVec(cell.grad, nor_vert )*0.3*(1.0-blend_factor)  );
    let nor = nor_vert;
    let dis = length(in.world_position * 0.005);
    let sun = normalize(-in.world_position);
    let sdn = dot(nor, sun);
    // let dif = max(0.0, sdn + dot(cell.grad, sun) * abs(sdn) * 0.6  );
    let dif = max(0.0, dot(nor, sun));
    let frez = 0.0*fresnel(rd, nor, 5.0);
    let refl = reflect(rd, nor);
    let spe = pow(max(0.0, dot(sun, refl)), 1.0)*0.1;
    // var col = vec3(0.5);
    var ems = vec3(0.0);
    if max(abs(col.r - 0.156862), max(
        abs(col.g - 0.85883),
        abs(col.b - 1.0)
    )) < 0.22 {
        ems += col * 20.0;
    }
    // let env = textureSample(background,diff_sampler, refl ).rgb * col;
    let difs = max(0.0, dif + 0.01) * 0.5 / (dis * dis);
    let scl = vec3(1.4, 1., 1.);
    let spec = (frez * 0.2 + 1.0) * spe  ;
    let color = col * difs + vec3(0.1, 0.2, 0.15) * (0.0 + frez) * 0.2 + ems + spec * difs;

    return vec4<f32>(color, 1.0)  ;
    // return vec4(0.00001);
    // return vec4(spe*(1.+frez));
    // return vec4(spe);
    // let pos = in.world_position % vec3(1.0) ;
    // return vec4(pos,1.0);
}
