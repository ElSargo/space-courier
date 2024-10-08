struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32, vert: VertexInput,
) -> VertexOutput {
    return VertexOutput(vec4(vert.position, 1.0, 1.0), vec2(vert.position.x, -vert.position.y) * 0.5 + 0.5);
}

