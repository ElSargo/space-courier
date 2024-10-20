use std::{collections::VecDeque, mem::size_of, sync::Arc};

use bevy_ecs::{
    component::Component,
    event::{Event, EventReader, EventWriter, Events},
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, In, Query, Res, ResMut, Resource},
};

use bytemuck::cast_slice;
use glam::{Vec2, Vec3Swizzles};

use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferBinding, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
    ComputePipeline, ComputePipelineDescriptor, Device, PushConstantRange, ShaderStages,
};

use crate::{
    ecs::Initailiser,
    gpu::{pack_buffers, Packable, RD, RQ},
    instance_buffer::InstanceBuffer,
    instance_layout,
    renderables::{SolidDrawable, SolidRenderable},
    shader_manager::{
        PipelineHandle, PipelinesView, ShaderLoader, SolidPipelineModuleResourceExtras,
        SolidShader, UnifiedPipelineModuleResource, UnifiedShaders,
    },
    transform::Transform,
    RawInstance,
};

type IndexType = u16;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Instance {
    position: [f32; 2],
    offset: u32,
}

instance_layout!(Instance,[0 => Float32x2 , 1 => Uint32]);

#[derive(Debug, Component)]
pub struct TriMesh {
    vertex_storage: Arc<wgpu::BindGroup>,
    index_buffer: wgpu::Buffer,
}

pub fn index_buffer(width: IndexType, device: &Device) -> wgpu::Buffer {
    assert!(width > 2);
    assert!(width * width < IndexType::MAX);
    assert_eq!(width % 2, 1);
    let mut idxs: Vec<IndexType> = Vec::with_capacity((width as usize - 1).pow(2) * 6);

    for y in 0..width - 1 {
        for x in 0..width - 1 {
            let this_row_base = x + width * y;
            let next_row_base = x + width * (y + 1);
            if y % 2 == 0 {
                idxs.extend_from_slice(&[
                    this_row_base,
                    next_row_base,
                    this_row_base + 1,
                    //
                    next_row_base,
                    next_row_base + 1,
                    this_row_base + 1,
                ]);
            } else {
                idxs.extend_from_slice(&[
                    next_row_base,
                    next_row_base + 1,
                    this_row_base,
                    //
                    this_row_base,
                    next_row_base + 1,
                    this_row_base + 1,
                ]);
            }
        }
    }

    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Tri mesh index_buffer"),
        usage: BufferUsages::INDEX,
        contents: bytemuck::cast_slice(idxs.as_slice()),
    })
}

pub fn init_mod(i: &mut Initailiser) {
    i.init
        .add_systems(init_res.after(crate::background::init_background));
    i.prepare_buffers
        .add_systems(pack_buffers::<Instance, (), With<TriMeshInstanceBufferMarker>>);
    i.update.add_systems((/*test,*/ gen_height, regen_meshes));
    i.world.init_resource::<Events<ReGenMeshes>>();
}

impl Packable for Instance {
    type Input<'a> = (&'a Transform, &'a TriMeshMarker);
    fn pack<'a>(input: Self::Input<'a>, eye: Vec2) -> Self {
        Self {
            position: (input.0.translation.xz() - eye).to_array(),
            offset: input.1.offset * CHUNK_SIZE as u32,
        }
    }
}

impl SolidDrawable for TriMesh {
    type Resource = ();
    const INSTANCE_BUFFER_SLOT: u32 = 0;
    fn draw<'a: 'b, 'b>(
        &self,
        render_pass: &mut wgpu::RenderPass,
        instances: std::ops::Range<u32>,
        _asset: &'a Self::Resource,
    ) {
        // TODO set instance buffer properly
        // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(3, &self.vertex_storage, &[]);
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_push_constants(ShaderStages::VERTEX, 0, &(CHUNK_DIM as u32).to_ne_bytes());
        render_pass.draw_indexed(0..self.index_buffer.size() as u32 / 2, 0, instances);
    }

    fn aquire(&self) -> Option<Arc<Self::Resource>> {
        Some(Arc::new(()))
    }
}

const CHUNKS: usize = 3;
const CHUNK_DIM: usize = 251;
const CHUNK_SIZE: usize = CHUNK_DIM * CHUNK_DIM;
const TOTAL_CHUNK_DATA_SIZE: usize = size_of::<f32>() * CHUNKS * CHUNK_SIZE;

#[derive(Event)]
struct ReGenMeshes;

pub fn init_res(shader_manager: Res<ShaderLoader>, mut commands: Commands, device: RD) {
    let output_compute_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
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
        bind_group_layouts: &[&output_compute_layout],
        push_constant_ranges: &[PushConstantRange {
            stages: ShaderStages::COMPUTE,
            range: 0..size_of::<Push>() as u32,
        }],
    });
    let compute_pipeline = shader_manager.add_compute_pipeline(
        UnifiedShaders {
            handle: shader_manager.load_shader("src/tri_mesh_gen.wgsl"),
        },
        move |In(mo): In<UnifiedPipelineModuleResource>,
              device: RD,
              mut ew: EventWriter<ReGenMeshes>| {
            ew.send(ReGenMeshes);
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

    let output_vertex_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Background outpus buffer layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Tri mesh vertex_buffer"),
        usage: BufferUsages::STORAGE,
        size: TOTAL_CHUNK_DATA_SIZE as u64,
        mapped_at_creation: false,
    });

    let output_compute_bind = Arc::new(device.create_bind_group(&BindGroupDescriptor {
        label: Some("Tri mesh storage bind"),
        layout: &output_compute_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: None,
            }),
        }],
    }));

    let output_vertex_bind = Arc::new(device.create_bind_group(&BindGroupDescriptor {
        label: Some("Tri mesh uniform bind"),
        layout: &output_vertex_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(BufferBinding {
                buffer: &buffer,
                offset: 0,
                size: None,
            }),
        }],
    }));

    let tq = TriQueue {
        queue: VecDeque::new(),
        pipeline: compute_pipeline,
        bind: output_compute_bind.clone(),
    };

    let mesh = TriMesh {
        vertex_storage: output_vertex_bind,
        index_buffer: index_buffer(CHUNK_DIM as u16, &device),
    };

    #[derive(Resource)]
    struct S(Arc<BindGroupLayout>);

    commands.insert_resource(S(output_vertex_layout.into()));

    commands.spawn((
        SolidRenderable {
            pipeline: shader_manager.add_render_pipeline(
                SolidShader {
                    handle: shader_manager.load_shader("./src/tri_mesh.wgsl"),
                },
                move |vertex_storage_buffer_bgl: Res<S>| SolidPipelineModuleResourceExtras {
                    vertex_buffers: Arc::new([Instance::desc()]),
                    extra_bind_groups: vec![vertex_storage_buffer_bgl.0.clone()],
                    label: Some("Tri mesh pipeline"),
                    push_constants: vec![PushConstantRange {
                        stages: ShaderStages::VERTEX,
                        range: 0..std::mem::size_of::<u32>() as u32,
                    }],
                },
            ),
            renderable: Box::new(mesh),
        },
        InstanceBuffer::new_init(
            Some("Tri mesh index buffer".to_string()),
            BufferUsages::COPY_DST | BufferUsages::VERTEX,
            &device,
            &[RawInstance::from_transform(&Transform::default())],
        ),
        TriMeshInstanceBufferMarker,
    ));

    commands.insert_resource(tq);
    commands.spawn((Transform::default(), TriMeshMarker { offset: 0 }));
    // commands.spawn((
    //     Transform {
    //         translation: vec3(CHUNK_DIM as f32, 0.0, 0.0),
    //         ..Default::default()
    //     },
    //     TriMeshMarker { offset: 1 },
    // ));
    // commands.spawn((
    //     Transform {
    //         translation: vec3(0.0, 0.0, CHUNK_DIM as f32 * 3.0f32.sqrt() / 2.0),
    //         ..Default::default()
    //     },
    //     TriMeshMarker { offset: 2 },
    // ));
}

#[derive(Component)]
pub struct TriMeshMarker {
    offset: u32,
}

#[derive(Component)]
pub struct TriMeshInstanceBufferMarker;

#[derive(Resource)]
pub struct TriQueue {
    queue: VecDeque<(Vec2, u32)>,
    pipeline: PipelineHandle,
    bind: Arc<wgpu::BindGroup>,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Push {
    offset: u32,
    size: u32,
    position: Vec2,
}

fn gen_height(
    device: RD,
    queue: RQ,
    pipelines: Res<PipelinesView<ComputePipeline>>,
    mut to_process: ResMut<TriQueue>,
    // mut commands: Commands,
) {
    if let Some(p) = pipelines.get(to_process.pipeline) {
        let mut enc = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Tri mesh enc"),
        });
        while let Some((position, offset_index)) = to_process.queue.pop_front() {
            println!("Dispatcing compute");
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Tri mesh compute"),
                timestamp_writes: None,
            });

            pass.set_bind_group(0, &to_process.bind, &[]);
            pass.set_pipeline(&p);
            let work = (CHUNK_DIM as u32).div_ceil(16);
            pass.set_push_constants(
                0,
                cast_slice(&[Push {
                    offset: CHUNK_SIZE as u32 * offset_index,
                    size: CHUNK_DIM as u32,
                    position,
                }]),
            );
            pass.dispatch_workgroups(work, work, 1);
        }
        queue.submit(std::iter::once(enc.finish()));
    }
}

fn regen_meshes(
    mut er: EventReader<ReGenMeshes>,
    mut tq: ResMut<TriQueue>,
    meshes: Query<(&Transform, &TriMeshMarker)>,
) {
    if let Some(_) = er.read().last() {
        for (transform, marker) in meshes.iter() {
            tq.queue
                .push_front((transform.translation.xz(), marker.offset));
        }
    }
}
