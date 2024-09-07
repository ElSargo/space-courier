@group(0) @binding(0) var<uniform> camera: Camera;

@group(1) @binding(0) var diff: texture_2d<f32>;
@group(1) @binding(1) var diff_sampler: sampler;


@group(2) @binding(0) var background: texture_cube<f32>;
@group(2) @binding(1) var background_sampler: sampler;

// @group(2) @binding(0) var<uniform> model: Model;

// struct Model {
//     matrix: mat4x4<f32>,
// };

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
    @location(1) relative_position: vec3<f32>,
    @location(2) world_normal: vec3<f32>,
    @location(3) distance: f32,
    @location(4) mesh_position: vec3<f32>,
    @location(5) barry: vec2<f32>,
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
    out.relative_position = vert_position.xyz;
    out.world_normal = rotation_matrix * vert.normal;
    out.distance = distance(vert_position.xyz, camera.view_pos.xyz);
    out.clip_position = camera.view_proj * vert_position;
    out.uv = vert.uv;
    out.mesh_position = vert.position;

    let bi = in_vertex_index - (in_vertex_index / 3u) * 3u;

    out.barry = vec2<f32>( f32(bi == 0u), f32(bi == 1u)  );
    
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
                let h = smoothstep(-1.0, 1.0, (dist - nd) / sm);
                dist = mix(dist, nd, h) - h * (1.0 - h) * sm / (1.0 + 3.0 * sm);
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
                let h = smoothstep(-1.0, 1.0, (dist - nd) / sm);
                dist = mix(dist, nd, h) - h * (1.0 - h) * sm / (1.0 + 3.0 * sm);
            }
        }
    }
    // dist = exp(-dist * 3.0);
    return dist;
}

fn drawObject(v: vec3f) -> f32 {
    let p = abs(fract(v) - 0.5);
    return max(max(p.x, p.y), p.z);
}

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

fn pow2(x: f32) -> f32 {
    return x * x;
}


fn reshape_inner_ring_diffuse(d: f32) -> f32 {
    return (1.0 - d) * d * (d + 0.2) * 5.15;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos_realative = in.relative_position;
    let world_position = in.relative_position + camera.view_pos.xyz;
    let rd = normalize(pos_realative);
    let color_sample = textureSample(diff, diff_sampler, in.uv) ;
    // let color_sample = vec4(0.2,0.2,0.2,1.0) ;
    let albedo = color_sample.rgb * color_sample.a;
    let ems = color_sample.rgb * mix(20.0, 0.0, color_sample.a);


    let nor = normalize(in.world_normal);
    // let nor_flat = normalize(cross(dpdx(in.world_position  + disp), dpdy(in.world_position + disp)));
    let dis = length(in.relative_position* 0.005);
    let freznel = fresnel(rd, nor, 5.0);
    let reflect_direction = reflect(rd, nor);

    //Light sources
    var color = vec3(0.0);
    // Sun
        {
        let light_color = vec3(0.2, 0.4, 1.0);
        let light_direction = normalize(-world_position);
        let distance_scaled = length(world_position * 0.004);
        let light_intensity = 1.0 / (distance_scaled * distance_scaled);
        let light_dot_normal = dot(nor, light_direction);

        let angle_atenuation = 1.0 - exp(-1.5 * distance_scaled);

        let diffuse = max(0.0, (1.0 - angle_atenuation) * 0.3 + dot(nor, light_direction)); 
        // let diffuse = max(0.0,light_dot_normal );
        let specular = (8.0 * angle_atenuation + 0.5) * pow(max(0.0, dot(light_direction, reflect_direction)), (8.0 + angle_atenuation)) * (0.05 + 0.95 * freznel) * diffuse;

        color += albedo * light_color * light_intensity * diffuse;
        color += light_intensity * sqrt(light_color) * specular;
    }
    // Ecretion disk
        {
        let light_color = vec3(1.0, 0.3, 0.05);
        let light_direction = vec3(0.0, -1.0, 0.0); 
        // let distance_scaled = length(in.world_position * 0.002);
        let light_intensity = 2.0;
        let light_dot_normal = dot(nor, light_direction);
        let diffuse = exp(-4.0 * pow2(1.0 + light_dot_normal - 2.0));
        let specular = 3.0 * pow(max(0.0, dot(light_direction, reflect_direction)), 8.0) * (0.05 + 0.95 * freznel);

        color += sqrt(albedo) * (light_color * light_color) * light_intensity * diffuse;
        color += light_intensity * (light_color) * specular;
    }

    // Ecretion disk inner ring
        {
        let light_color = vec3(1.0, 0.0, 0.15);
        let light_direction = normalize(vec3(-1.0, 0.7, 0.0)); 
        // let distance_scaled = length(in.world_position * 0.002);
        let light_intensity = 0.7;
        let light_dot_normal = max(0.0, dot(nor, light_direction));
        let diffuse = reshape_inner_ring_diffuse(light_dot_normal);

        let specular = 0.3 * pow(max(0.0, dot(light_direction, reflect_direction)), 8.0) * (0.05 + 0.95 * freznel);

        color += sqrt(albedo) * (light_color * light_color) * light_intensity * diffuse;
        color += light_intensity * sqrt(light_color) * specular;
    }

    // Emission 
        {
        color += ems;
    }


    // Stars / background 
        {
        let light_color = textureSample(background, diff_sampler, reflect_direction).rgb;
        color += freznel * light_color   ;
    }


    // return vec4<f32>(color, 1.0)  ;

    let edge = max(
        max(in.barry.x,in.barry.y ),
        1.0 - in.barry.x - in.barry.y
    );
    
    return vec4(vec3(1.0 - pow(edge,10.)),1.0);
}

// Stars / background 
// {
//     // let light_color = textureSample(background,diff_sampler, reflect_direction).rgb; 
//     // color += freznel * light_color  * blend_factor;

//     var diff = vec3(0.0);
//     var spec = textureSample(background,diff_sampler, reflect_direction).rgb * 0.2;
//     for (var dx=-1.0;dx<1.5;dx += 0.5) {
//     for (var dy=-1.0;dy<1.5;dy += 0.5) {
//     for (var dz=-1.0;dz<1.5;dz += 0.5) {
//         let offset = vec3(dx,dy,dz);
//         let sd = normalize(nor + offset * 0.4 );
//         let sample = textureSample(background,diff_sampler, sd).rgb;
//         diff += sample *  dot(nor, sd);
//         spec += sample * pow(dot(reflect_direction, sd), 8.0) * 20.0 ;
//     }}}

//     color += ( (albedo )* diff + spec*freznel)*0.05;
// }
