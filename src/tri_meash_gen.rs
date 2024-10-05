use std::{any::Any, collections::VecDeque, sync::Arc};

use bevy_ecs::{
    component::Component,
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, In, Res, ResMut, Resource},
};

use bytemuck::cast_slice;
use glam::{vec2, vec3, vec4, Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferBinding, BufferUsages, CommandEncoderDescriptor, ComputePipeline,
    ComputePipelineDescriptor, Device, ShaderStages,
};

use crate::{
    ecs::Initailiser,
    gpu::{pack_buffers, InstanceBuffer, Packable, SolidDrawable, SolidRenderable, RD, RQ},
    shader_manager::{
        PipelineHandle, PipelinesView, ShaderManagerCtrl, SolidPipelineModuleResourceExtras,
        SolidShader, UnifiedPipelineModuleResource, UnifiedShaders,
    },
    transform::Transform,
    vertex_layout, RawInstance,
};

type IndexType = u16;

#[derive(Debug)]
pub struct FlatMesh {
    indices: Vec<IndexType>,
    positions: Vec<Vec2>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    position: [f32; 4],
}
vertex_layout!(Vertex,[0 => Float32x4 ]);

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Instance {
    position: [f32; 2],
}
vertex_layout!(Instance,[0 => Float32x2 ]);

#[derive(Debug, Component)]
pub struct TriMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

pub fn gen_mesh(width: usize, depth: usize) -> FlatMesh {
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
            positions.push(vec2(x, 0.0));
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
                    positions.push(vec2(x, y));
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
                    i + row_short_idx,
                    i + row_previous_idx + 1,
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

    FlatMesh {
        indices: idxs,
        positions,
    }
}

pub fn init_mod(i: &mut Initailiser) {
    i.init
        .add_systems(init_res.after(crate::background::init_background));
    i.prepare_buffers
        .add_systems(pack_buffers::<Vec2, With<TriMeshMarker>, With<TriMeshMarker>>);
    i.update.add_systems(gen_height);
}

impl Packable for Vec2 {
    type Input<'a> = &'a Transform;
    fn pack<'a>(input: Self::Input<'a>, eye: Vec2) -> Self {
        input.translation.xz() - eye
    }
}

impl SolidDrawable for TriMesh {
    fn draw<'a: 'b, 'b>(
        &self,
        render_pass: &mut wgpu::RenderPass,
        _instances: std::ops::Range<u32>,
        _asset: &'a dyn Any,
    ) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_buffer.size() as u32 / 2, 0, 0..1);
    }

    fn aquire(&self) -> Option<Arc<dyn Any>> {
        Some(Arc::new(()))
    }
}

pub fn init_res(shader_manager: Res<ShaderManagerCtrl>, mut commands: Commands, device: RD) {
    let shader = shader_manager.load_shader("./src/tri_mesh.wgsl");

    let flat_mesh = gen_mesh(100, 100, |x| terain_height(x));
    let inputs_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Background inputs buffer layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let output_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Background outpus buffer layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Tri mesh gen layout"),
        bind_group_layouts: &[&inputs_layout, &output_layout],
        push_constant_ranges: &[],
    });

    let compute_pipeline = shader_manager.add_compute_pipeline(
        UnifiedShaders {
            handle: shader_manager.load_shader("src/tri_mesh_gen.wgsl"),
        },
        move |In(mo): In<UnifiedPipelineModuleResource>, device: RD| {
            device.create_compute_pipeline(&ComputePipelineDescriptor {
                label: Some("Tri mesh generator"),
                layout: Some(&pipeline_layout),
                module: &mo,
                entry_point: "full",
                compilation_options: Default::default(),
                cache: None,
            })
        },
    );

    let mut tq = TriQueue {
        queue: VecDeque::new(),
        pipeline: compute_pipeline,
    };

    let mesh = Arc::new(prepare_mesh(
        &flat_mesh,
        &device,
        &inputs_layout,
        &output_layout,
        &mut tq,
    ));

    commands.spawn((
        SolidRenderable {
            pipeline_handle: shader_manager.add_render_pipeline(
                SolidShader { handle: shader },
                move || SolidPipelineModuleResourceExtras {
                    vertex_buffers: Arc::new([Vertex::desc(), Vertex::instance_desc()]),
                    extra_bind_groups: vec![],
                },
            ),
            renderable: Box::new(TriMeshHandle(mesh)),
        },
        InstanceBuffer::new_init(
            Some("Tri mesh index buffer".to_string()),
            BufferUsages::COPY_DST | BufferUsages::VERTEX,
            &device,
            &[RawInstance::from_transform(&Transform::default())],
        ),
        TriMeshMarker,
    ));

    commands.insert_resource(tq);
    commands.spawn((Transform::default(), TriMeshMarker));
}

#[derive(Component)]
pub struct TriMeshMarker;

pub fn prepare_mesh(
    tri_mesh: &FlatMesh,
    device: &Device,
    input_layout: &BindGroupLayout,
    output_layout: &BindGroupLayout,
    tri_q: &mut TriQueue,
) -> TriMesh {
    let inputs = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Tri mesh vertex_buffer"),
        usage: BufferUsages::STORAGE,
        contents: bytemuck::cast_slice(tri_mesh.positions.as_slice()),
    });

    let output = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Tri mesh vertex_buffer"),
        usage: BufferUsages::VERTEX | BufferUsages::STORAGE,
        contents: bytemuck::cast_slice(
            tri_mesh
                .positions
                .iter()
                .map(|xy| vec4(xy.x, 0.0, xy.y, 0.0))
                .collect::<Vec<_>>()
                .as_slice(),
        ),
    });

    let input_bind = Arc::new(device.create_bind_group(&BindGroupDescriptor {
        label: Some("Tri mesh storage layout"),
        layout: input_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(BufferBinding {
                buffer: &inputs,
                offset: 0,
                size: None,
            }),
        }],
    }));

    let output_bind = Arc::new(device.create_bind_group(&BindGroupDescriptor {
        label: Some("Tri mesh storage layout"),
        layout: output_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(BufferBinding {
                buffer: &output,
                offset: 0,
                size: None,
            }),
        }],
    }));

    tri_q.queue.push_front((
        tri_mesh.positions.len() as u32,
        IOL {
            i: input_bind,
            o: output_bind,
        },
    ));

    let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Tri mesh index_buffer"),
        usage: BufferUsages::INDEX,
        contents: bytemuck::cast_slice(tri_mesh.indices.as_slice()),
    });

    TriMesh {
        vertex_buffer: output,
        index_buffer,
        num_elements: 1,
    }
}

struct IOL {
    i: Arc<BindGroup>,
    o: Arc<BindGroup>,
}

#[derive(Resource)]
pub struct TriQueue {
    queue: VecDeque<(u32, IOL)>,
    pipeline: PipelineHandle,
}

fn gen_height(
    device: RD,
    queue: RQ,
    pipelines: Res<PipelinesView<ComputePipeline>>,
    mut to_process: ResMut<TriQueue>,
    // mut commands: Commands,
) {
    if let Some(p) = pipelines.get(&to_process.pipeline) {
        let mut enc = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Tri mesh enc"),
        });
        while let Some((count, bind)) = to_process.queue.pop_front() {
            println!("Dispatcing compute");
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Tri mesh compute"),
                timestamp_writes: None,
            });

            pass.set_bind_group(0, &bind.i, &[]);
            pass.set_bind_group(1, &bind.o, &[]);
            pass.set_pipeline(&p);
            pass.dispatch_workgroups((count / 256) + 1, 1, 1);
        }
        queue.submit(std::iter::once(enc.finish()));
    }
}

fn draw_object(v: Vec3) -> f32 {
    let p = (v.fract() - 0.5).abs();
    return p.x.max(p.y).max(p.z);
}

fn cell_tile(v: Vec3) -> f32 {
    let mut p = v;
    // Draw four overlapping objects (spheres, in this case) at various positions throughout the tile.
    let mut d = Vec4::ZERO;
    d.x = draw_object(p - vec3(0.81, 0.62, 0.53));
    p.x = (p.y - p.x) * 0.7071;
    p.y = (p.y + p.x) * 0.7071;
    //p.xy = vec2(p.y*.866025 - p.x*.5, p.y*.5 + p.x*.866025); // Etc.
    d.y = draw_object(p - vec3(0.39, 0.2, 0.11));
    p.y = (p.z - p.y) * 0.7071;
    p.z = (p.z + p.y) * 0.7071;
    //p.yz = vec2(p.z*.866025 - p.y*.5, p.z*.5 + p.y*.866025); // Etc.
    d.z = draw_object(p - vec3(0.62, 0.24, 0.06));
    p.x = (p.z - p.x) * 0.7071;
    p.z = (p.z + p.x) * 0.7071;
    //p.xz = vec2(p.z*.866025 - p.x*.5, p.z*.5 + p.x*.866025); // Etc.
    d.w = draw_object(p - vec3(0.2, 0.82, 0.64));

    // Obtain the minimum, and you're done.
    d.x = d.x.min(d.y);
    d.y = d.z.min(d.w);

    return 1. - d.x.min(d.y) * 2.; // Scale between zero and one... roughly.

    // For anyone wanting to experiment with this, the following gives better variance:
    //const float scale = 1.; // 1 up to 4, or higher, depending on the look you want.
    // Obviously, for the reverse, you take the one and minus away.
    //return 1. - min(min(d.x, d.y)*2.*scale, 1.);
}

fn terain_height(p: Vec2) -> f32 {
    let position = vec3(p.x, 0.0, p.y);
    let noise = cell_tile(position * 0.1);

    if noise > 0.70 {
        ((noise - 0.6) * 4.0).powi(2)
    } else {
        0.0
    }
}
