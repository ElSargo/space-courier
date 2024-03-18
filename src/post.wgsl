// Use with the full screen quad
@group(0) @binding(0) var color_texture: texture_2d<f32>;
@group(0) @binding(1) var color_sampler: sampler;
@group(0) @binding(2) var depth_texture: texture_depth_2d;
@group(0) @binding(3) var depth_sampler: sampler;

@group(1) @binding(0) var<uniform> camera: Camera;

@group(2) @binding(0) var bloom_texture: texture_2d<f32>;

@group(3) @binding(0) var planet_shadows_texture: texture_2d<f32>;

struct Camera {
    view_pos: vec4<f32>,
    view: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
};

struct VertexInput {
    // Clip postion
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};



fn aces_tonemap(color:vec3<f32>) -> vec3<f32>{	
	let m1 = mat3x3<f32>(
        0.59719, 0.07600, 0.02840,
        0.35458, 0.90834, 0.13383,
        0.04823, 0.01566, 0.83777
	);
	let m2 = mat3x3<f32>(
        1.60475, -0.10208, -0.00327,
        -0.53108,  1.10813, -0.07276,
        -0.07367, -0.00605,  1.07602
	);
	let  v = m1 * color;    
	let  a = v * (v + 0.0245786) - 0.000090537;
	let  b = v * (0.983729 * v + 0.4329510) + 0.238081;
	return clamp(m2 * (a / b), vec3(0.0),vec3( 1.0));	
}


fn gamma(col: vec3<f32>) -> vec3<f32>{
    return pow(col, vec3(1.0 / 2.2));
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32, vert: VertexInput,     
) -> VertexOutput {
    return VertexOutput( vec4(vert.position,1.), vert.uv );
}

fn fog_int(ro: vec3<f32>, rd: vec3<f32>, t: f32) -> f32 {
    let a = (rd.y * ro.z - ro.y * rd.z);
    let b = (rd.y * ro.z - ro.y * rd.z);
    let c = ro.x * rd.x + ro.y * rd.y + ro.z * rd.z + rd.x * rd.x * t + rd.y * rd.y * t + rd.z * rd.z * t ;
    let d = rd.x * rd.x * ( ro.y * ro.y + ro.z * ro.z ) + a * a - 2.0 * ro.x * rd.x * ( ro.y * rd.y + ro.z * rd.z ) + ro.x * ro.x * ( rd.y * rd.y + rd.z * rd.z ) ;
    let e =  rd.x * rd.x * ( ro.y * ro.y + ro.z * ro.z ) + b * b - 2.0 * ro.x * rd.x * ( ro.y * rd.y + ro.z * rd.z ) + ro.x * ro.x * ( rd.y * rd.y + rd.z * rd.z ) ;
    return atan( c / sqrt(d) ) / sqrt(e);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let depth_inv = textureSample(depth_texture, depth_sampler, in.uv);
    let lin_depth = 1.0/depth_inv;
    // No chromaitc abbr
    var color = textureSample(color_texture, color_sampler, in.uv).rgb;
    var bloom = textureSample(bloom_texture, color_sampler, in.uv).rgb;
    var pls = textureSample(planet_shadows_texture, color_sampler, in.uv).rg;


    let view_pos_homogeneous = camera.inv_proj * vec4((vec2(in.uv.x,1.0 - in.uv.y) - 0.5)*2.0,1.0,1.0);
    let view_ray_direction = view_pos_homogeneous.xyz / view_pos_homogeneous.w;
    var ray_direction = normalize((camera.inv_view * vec4(view_ray_direction, 0.0)).xyz);

    


    //Chromatic abbr
    // var c = in.uv - 0.5;
    // let st = 0.02;
    // var color = vec3(
    //     textureSample(color_texture, color_sampler, c*( 1.0 - st ) + 0.5).r,
    //     textureSample(color_texture, color_sampler, c*( 1.0      ) + 0.5).g,
    //     textureSample(color_texture, color_sampler, c*( 1.0 + st ) + 0.5).b
    // );
    // var bloom = vec3(
    //     textureSample(bloom_texture, color_sampler, c*( 1.0 - st ) + 0.5).r,
    //     textureSample(bloom_texture, color_sampler, c*( 1.0      ) + 0.5).g,
    //     textureSample(bloom_texture, color_sampler, c*( 1.0 + st ) + 0.5).b
    // );
    
    // Depth fog
    // color = mix( color, vec3(0.04,0.0,0.01), max(0.0,0.1*(1.0-exp(-lin_depth*0.0001) - 0.2*pls.x )));
    
    // return vec4(gamma(bloom),1.0);
    let fog_col = vec3(5.4,0.75,0.1);
    // let fog_col = vec3(0.2,5.0,1.2);
    let fog = max(0.001,10.*(fog_int(camera.view_pos.xyz, ray_direction, lin_depth) - fog_int(camera.view_pos.xyz, ray_direction, 0.0)) + pls.x + sin(in.uv.x*99999.0)/256.0+sin(in.uv.y*99999.0)/256.0+sin((in.uv.x+in.uv.y)*5000.0)/256.0);
    color = mix(color,fog*fog_col*0.1,1.0-exp(-lin_depth*0.0001));
    color = aces_tonemap(color);
    color += bloom;

    // color +=  pls.x  ;
    
    

    // color.r += pls.r;
    // color.g += pls.g;

    // color += pls.x;
    
    // color.g += pls.y  ;
    // let vignet_signal = smoothstep(vec2(1.),vec2(0.8), pow(abs(in.uv - vec2(0.5)) * vec2(2.0),vec2(2.0) ) );
    // color *= vignet_signal.x * vignet_signal.y;

    // mix(color,vec3(pls.r,pls.g,0.0),0.5);

    
    // if depth_inv <= 0.0 {
    //     color = ray_direction;
    // }

    
    return vec4(gamma(color), 1.0);
}

