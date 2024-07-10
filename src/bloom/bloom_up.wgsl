@group(0) @binding(0) var color_sampler: sampler;
@group(1) @binding(0) var color_texture: texture_2d<f32>;
@group(2) @binding(0) var low_res_texture: texture_2d<f32>;


struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
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
    color = mix(
        color,
        textureSample(color_texture, color_sampler, in.uv).rgb,
        0.7
    );
    
    return vec4(color , 1.0);
}


