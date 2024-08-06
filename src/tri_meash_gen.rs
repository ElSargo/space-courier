use std::mem::size_of;

use anymap::AnyMap;

use bytemuck::bytes_of;
use glam::{vec2, vec3, Vec2, Vec3};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, Device, FragmentState, PipelineLayout, PipelineLayoutDescriptor, TextureFormat,
    VertexState,
};

use crate::{
    background::Background,
    camera::CameraUniform,
    get_all,
    gpu::{Gpu, InstanceBuffer, InstanceBufferDescriptor},
    shader_manager::{PipelineHandle, ShaderHandle, ShaderManager},
    texture,
    transform::Transform,
    RawInstance, ViewTextureReasources,
};

type IndexType = u16;

#[derive(Debug)]
pub struct FlatMesh {
    indices: Vec<IndexType>,
    positions: Vec<Vec3>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3 ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

struct TriMeshResources {
    pub pipeline_handle: PipelineHandle,
}

pub struct TriMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: InstanceBuffer,
    num_elements: u32,
}

pub fn gen_mesh(
    width: usize,
    depth: usize,
    mut height_function: impl FnMut(Vec2) -> f32,
) -> FlatMesh {
    let num_chunks = (depth - 1) / 2;
    assert!(num_chunks > 0);
    assert!(width > 2);
    assert!(width * depth < IndexType::MAX as usize);
    let mut positions = Vec::with_capacity(width + num_chunks * (width * 2 - 1));
    let mut idxs: Vec<IndexType> = Vec::with_capacity(num_chunks * 3 * ((width - 2) * 4 + 2));
    // Positions
    {
        // Insert header
        // let dx = 1.0;
        let dy = 3_f32.sqrt() / 2.0;
        let mut x = 0.0;
        for _ in 0..width {
            let height = height_function(vec2(x, 0.0));
            positions.push(vec3(x, height, 0.0));
            x += 1.0;
        }
        // Chunks
        x = 0.0;
        let mut y = dy;
        for _ in 0..num_chunks {
            // Add positions
            for o in 0..=1 {
                let mut x = x + if o == 0 { 0.5 } else { 0.0 };
                for _ in 0..width - 1 + o {
                    let height = height_function(vec2(x, y));
                    positions.push(vec3(x, height, y));
                    x += 1.0;
                }
                // x = 0.0;
                y += dy;
            }
        }
    }

    // Idxs
    {
        let width = width as IndexType;
        let num_chunks = num_chunks as IndexType;
        let idxs_per_chunk = 2 * width - 1;
        for chunk in 0..num_chunks {
            // Indexes of the starts of the relavant rows
            let (row_previous_idx, row_short_idx, row_long_idx) = (
                (chunk * idxs_per_chunk),
                (chunk * idxs_per_chunk + width),
                (chunk * idxs_per_chunk + (2 * width - 1)),
            );
            // Inner quads
            for i in 0..width - 2 {
                let a = [
                    i + row_previous_idx,
                    i + row_previous_idx + 1,
                    i + row_short_idx,
                    // i + row_short_idx + 1,
                ];
                let b = [
                    i + row_short_idx,
                    i + row_short_idx + 1,
                    i + row_previous_idx + 1,
                ];
                let c = [
                    i + row_short_idx,
                    i + row_long_idx + 1,
                    i + row_short_idx + 1,
                ];
                let d = [i + row_short_idx, i + row_long_idx, i + row_long_idx + 1];
                let data: [IndexType; 12] = unsafe { std::mem::transmute([a, b, c, d]) };
                idxs.extend_from_slice(&data);
            }
            // End tris
            let top = [
                row_previous_idx + width - 2,
                row_previous_idx + width - 1,
                row_short_idx + width - 2,
            ];
            let bottom = [
                row_short_idx + width - 2,
                row_long_idx + width - 2,
                row_long_idx + width - 1,
            ];
            let chunk_end_idxs: [IndexType; 6] = unsafe { std::mem::transmute([bottom, top]) };
            idxs.extend_from_slice(&chunk_end_idxs)
        }
    }

    // let old_idxs = idxs;
    // let old_positions = positions;
    // let mut positions = Vec::with_capacity(old_positions.len() * 3);
    // let mut idxs = Vec::with_capacity(old_positions.len() * 3);

    // for position in old_positions {
    //     let disp = 0.3;
    //     let base_idx = positions.len() as INDEX_TYPE;
    //     positions.extend_from_slice(&[
    //         position + vec3(-disp, 0.0, -disp),
    //         position + vec3(0.0, disp, 0.0),
    //         position + vec3(-disp, 0.0, disp),
    //     ]);

    //     idxs.extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);
    // }

    // for is in idxs.windows(3) {
    //     let [a, b, c] = [
    //         positions[is[0] as usize],
    //         positions[is[1] as usize],
    //         positions[is[2] as usize],
    //     ];

    //     let ab = a.distance(b);
    //     let ac = a.distance(c);
    //     let bc = b.distance(c);

    //     if let Err(_) = catch_unwind(|| {
    //         assert!((ab - ac).abs() < 0.001);
    //         assert!((ab - bc).abs() < 0.001);
    //         assert!((bc - ac).abs() < 0.001);
    //     }) {
    //         // panic!("ab: {ab:?} ac: {ac:?} bc: {bc:?}; a: {a:?} b: {b:?} c: {c:?}")
    //     }
    // }

    FlatMesh {
        indices: idxs,
        positions,
    }
}

pub fn init(shader_manager: &mut ShaderManager, rr: &mut AnyMap, device: &Device) {
    let shader = shader_manager.load_shader("./src/tri_mesh.wgsl");
    let (camera, bg) = get_all!(rr, CameraUniform, Background).unwrap();
    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("tri mesh pipeline layout"),
        bind_group_layouts: &[
            &camera.inner.bind_group_layout,
            &bg.texture_cube_bind_group_layout,
        ],
        push_constant_ranges: &[],
    });
    struct TriMeshShaderResources(ShaderHandle, PipelineLayout);
    rr.insert(TriMeshShaderResources(shader, pipeline_layout));
    let pipeline_handle = shader_manager.add_render_pipeline(
        vec![shader],
        Box::new(|rr, device, sm| -> wgpu::RenderPipeline {
            let (TriMeshShaderResources(shader_handle, pipeline_layout),) =
                get_all!(rr, TriMeshShaderResources).unwrap();
            let shader_module = sm.get_module(*shader_handle).unwrap();
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Tri mesh render pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader_module,
                    entry_point: "vs_main",
                    buffers: &[Vertex::desc(), RawInstance::desc()],
                },
                fragment: Some(FragmentState {
                    module: &shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        // 4.
                        format: TextureFormat::Rgba16Float,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Greater, // 1.
                    stencil: wgpu::StencilState::default(),        // 2.
                    bias: wgpu::DepthBiasState::default(),
                }), // 1.
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw, // 2.
                    // cull_mode: Some(wgpu::Face::Back),
                    cull_mode: None,
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                multisample: wgpu::MultisampleState {
                    count: 1,                         // 2.
                    mask: !0,                         // 3.
                    alpha_to_coverage_enabled: false, // 4.
                },
                multiview: None, // 5.
            })
        }),
    );

    rr.insert(TriMeshResources { pipeline_handle });
}

pub fn prepare_mesh(tri_mesh: &FlatMesh, gpu: &Gpu) -> TriMesh {
    // println!("{tri_mesh:#?}");
    let vertex_buffer = gpu.device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Tri mesh vertex_buffer"),
        usage: BufferUsages::VERTEX,
        contents: bytemuck::cast_slice(tri_mesh.positions.as_slice()),
    });
    let index_buffer = gpu.device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Tri mesh index_buffer"),
        usage: BufferUsages::INDEX,
        contents: bytemuck::cast_slice(tri_mesh.indices.as_slice()),
    });
    let buffer_usages = BufferUsages::COPY_DST | BufferUsages::VERTEX;
    let instance_buffer = gpu.device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Tri mesh instance_buffer"),
        usage: buffer_usages,
        contents: bytes_of(&RawInstance::from_transform(&Transform::default())),
    });

    TriMesh {
        vertex_buffer,
        index_buffer,
        instance_buffer: InstanceBuffer {
            descriptor: InstanceBufferDescriptor {
                label: Some("Tri mesh index buffer".to_string()),
                usages: buffer_usages,
                mapped_at_creation: true,
            },
            buffer: instance_buffer,
            instances: 1,
        },
        num_elements: tri_mesh.indices.len() as u32,
    }
}

pub fn draw_meshes(
    encoder: &mut wgpu::CommandEncoder,
    shader_manager: &ShaderManager,
    meshes: &[TriMesh],
    rr: &AnyMap,
) -> Option<()> {
    let (
        TriMeshResources {
            pipeline_handle, ..
        },
        ViewTextureReasources {
            view_texture,
            depth_texture,
            ..
        },
        camera,
        Background {
            texture_cube_bind_group,
            ..
        },
    ) = get_all!(
        rr,
        TriMeshResources,
        ViewTextureReasources,
        CameraUniform,
        Background
    )
    .expect("Didn't capture required resources");

    if let Some(pipeline) = shader_manager.get_render_pipeline(&pipeline_handle) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Tri mesh render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view_texture.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &camera.inner.bind_group, &[]);
        render_pass.set_bind_group(1, &texture_cube_bind_group, &[]);
        for mesh in meshes {
            render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(1, mesh.instance_buffer.buffer.slice(..));
            render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..mesh.num_elements, 0, 0..mesh.instance_buffer.instances);
        }
    }
    Some(())
} // pub fn draw<'a>(&'a self, render_pass: &mut RenderPass<'a>) {
  // }
