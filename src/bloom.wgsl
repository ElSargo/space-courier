@group(0) @binding(0) var color_sampler: sampler;
@group(1) @binding(0) var<uniform> threshold: f32;
@group(2) @binding(0) var color_texture: texture_2d<f32>;
@group(3) @binding(0) var low_res_texture: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn dot2(v: vec2<f32>) -> f32 {
    return dot(v,v);
}

fn dot3(v: vec3<f32>) -> f32 {
    return dot(v,v);
}
    // 8   9   10
    //   3   4
    // 7   1   11
    //   2   5
    // 6  13   12

const arr_size = 13u;
const offsets = array<vec2<f32>,arr_size>(
    vec2(0.0,0.0),   // 1
    vec2(-0.5,-0.5), // 2
    vec2(-0.5,0.5),  // 3
    vec2(0.5,0.5),   // 4
    vec2(0.5,-0.5),  // 5
    vec2(-1.0,-1.0), // 6
    vec2(-1.0,0.0),  // 7
    vec2(-1.0,1.0),  // 8
    vec2(0.0,1.0),   // 9
    vec2(1.0,1.0),   // 10
    vec2(1.0,0.0),   // 11
    vec2(1.0,-1.0),  // 12
    vec2(0.0,-1.0),  // 13
);

const weights = array<f32,arr_size>(
    0.125, // 1
    0.125, // 2
    0.125, // 3
    0.125, // 4
    0.125, // 5
    0.03125, // 6
    0.0625, // 7
    0.03125, // 8
    0.0625, // 9
    0.03125, // 10
    0.0625, // 11
    0.03125, // 12
    0.0625, // 13
);

@fragment
fn down(in: VertexOutput) -> @location(0) vec4<f32> {
    let dxdy = vec2(dpdx(in.uv.x),dpdy(in.uv.y));
    var color = vec3(0.0);
    var offset = offsets[0];
    var weight = weights[0];
    var sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    var sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[1];
    weight = weights[1];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[2];
    weight = weights[2];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[3];
    weight = weights[3];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[4];
    weight = weights[4];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[5];
    weight = weights[5];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[6];
    weight = weights[6];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[7];
    weight = weights[7];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[8];
    weight = weights[8];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[9];
    weight = weights[9];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[10];
    weight = weights[10];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[11];
    weight = weights[11];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    offset = offsets[12];
    weight = weights[12];
    sample_position = in.uv + offset * dxdy; // Texture we are sampling is twice as big;
    sample = textureSample(color_texture, color_sampler, sample_position).rgb;
    color += max(vec3(0.0), sample - threshold) * weight;
    return vec4(color * 1.0, 1.0);
}


@fragment
fn up(in: VertexOutput) -> @location(0) vec4<f32> {
    // let same_res = textureSample(color_texture, color_sampler, in.uv).rgb;
    // let low_res = textureSample(low_res_texture, color_sampler, in.uv).rgb;
    // var color = vec3(0.0);
    let pi = 3.141592653589793238462643383279502884197;
    var a = 0.0;
    let dxdy = vec2(dpdx(in.uv.x),dpdy(in.uv.y)*1.0);
    var color = vec3(0.0);
    for (var dx = -1.0; dx < 1.5 ; dx += 1.0 ) {
    for (var dy = -1.0; dy < 1.5 ; dy += 1.0 ) {
        let offset = vec2(dx,dy);
        let sample_position = in.uv + offset * dxdy * pi;
        let sample = textureSample(low_res_texture, color_sampler, sample_position).rgb;
        let e = 2.0 * (1.0 - 0.5*abs(dx)) * (1.0 - 0.5*abs(dy));
        color += sample * e ;
        a += e;
    }}
    color /= a;
    // color *= 0.5;
    color = mix(
        color,
        textureSample(color_texture, color_sampler, in.uv).rgb,
        0.7
    );
    
    return vec4(color , 1.0);
}


