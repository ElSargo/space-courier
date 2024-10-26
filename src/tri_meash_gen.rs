use crate::instance_buffer;
use std::{collections::VecDeque, mem::size_of, sync::Arc};

use bevy_ecs::{
    component::Component,
    event::{Event, EventReader, EventWriter, Events},
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, In, Query, Res, ResMut, Resource},
};

use bytemuck::cast_slice;
use glam::{vec2, IVec2, Vec2, Vec3Swizzles};

use crate::{
    ecs::Initailiser,
    gpu::{pack_buffers, Packable, RD, RQ},
    instance_layout,
    renderables::{SolidDrawable, SolidRenderable},
    shader_manager::{
        PipelineHandle, ShaderLoader, SolidPipelineModuleResourceExtras, SolidShader,
    },
    Camera, ComputePipelines,
};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BufferBinding, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
    ComputePipelineDescriptor, Device, PushConstantRange, ShaderModule, ShaderStages,
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
    i.init.add_systems(
        (
            init_res.after(crate::background::init_background),
            init_meshes,
        )
            .chain(),
    );
    i.prepare_buffers
        .add_systems(pack_buffers::<Instance, (), With<TriMeshInstanceBufferMarker>>);
    i.update.add_systems((
        /*test,*/
        gen_height,
        // regen_meshes,
        mangage_mesh_positions.after(regen_meshes),
    ));
    i.world.init_resource::<Events<ReGenMeshes>>();
}

impl Packable for Instance {
    type Input<'a> = &'a TriMeshMarker;
    fn pack<'a>(input: Self::Input<'a>, eye: Vec2) -> Option<Self> {
        let bounding_circle = vec2(
            CHUNK_DIM as f32 - 1.0,
            (CHUNK_DIM as f32 - 1.0) * 3f32.sqrt() / 2.0,
        )
        .length();

        // let forward = eye

        Some(Self {
            position: (chunk_id_to_position(input.computed_chunk_id) - eye).to_array(),
            offset: input.offset * CHUNK_SIZE as u32,
        })
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

const RENDER_DISTANCE: usize = 20;
const CHUNKS: usize = (RENDER_DISTANCE) * (RENDER_DISTANCE) * 4;
const CHUNK_DIM: usize = 33;
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
        shader_manager.load_shader("src/tri_mesh_gen.wgsl"),
        move |mo: In<Arc<ShaderModule>>, device: RD, mut ew: EventWriter<ReGenMeshes>| {
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

    commands.insert_resource(TriQueue {
        queue: VecDeque::new(),
        pipeline: compute_pipeline,
        bind: output_compute_bind.clone(),
    });

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
        instance_buffer!(@ "Tri mesh index buffer", &device),
        TriMeshInstanceBufferMarker,
    ));
}

#[derive(Component, Clone, Copy)]
pub struct TriMeshMarker {
    base_chunk_id: IVec2,
    computed_chunk_id: IVec2,
    offset: u32,
}

#[derive(Component)]
pub struct TriMeshInstanceBufferMarker;

#[derive(Resource)]
pub struct TriQueue {
    queue: VecDeque<TriMeshMarker>,
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
    pipelines: Res<ComputePipelines>,
    mut to_process: ResMut<TriQueue>,
    // mut commands: Commands,
) {
    if let Some(p) = pipelines.get(to_process.pipeline) {
        let mut enc = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Tri mesh enc"),
        });
        while let Some(TriMeshMarker {
            base_chunk_id: _,
            computed_chunk_id,
            offset,
        }) = to_process.queue.pop_front()
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Tri mesh compute"),
                timestamp_writes: None,
            });
            let offset_position = CHUNK_SIZE as u32 * offset;
            let size = CHUNK_DIM as u32;
            let position = chunk_id_to_position(computed_chunk_id);
            // println!("Dispatcing compute: pos: {position:?}, offset_index: {offset:?} offset_position: {offset_position}");

            pass.set_bind_group(0, &to_process.bind, &[]);
            pass.set_pipeline(&p);
            let work = (CHUNK_DIM as u32).div_ceil(16);
            pass.set_push_constants(
                0,
                cast_slice(&[Push {
                    offset: offset_position,
                    size,
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
    meshes: Query<&TriMeshMarker>,
) {
    if let Some(_) = er.read().last() {
        for marker in meshes.iter() {
            tq.queue.push_front(marker.clone());
        }
    }
}

pub fn positon_to_chunk_id(position: Vec2) -> IVec2 {
    let denomenator: Vec2 = vec2(1.0, 3f32.sqrt() / 2.0) * (CHUNK_DIM - 1) as f32;
    let scaled = position / denomenator;
    IVec2::new(scaled.x.round() as i32, scaled.y.round() as i32)
}

fn chunk_id_to_position(position: IVec2) -> Vec2 {
    let scaled = vec2(position.x as f32, position.y as f32);
    let scaler: Vec2 = vec2(1.0, 3f32.sqrt() / 2.0) * (CHUNK_DIM - 1) as f32;
    scaled * scaler
}

fn mangage_mesh_positions(
    mut tri_meshes: Query<&mut TriMeshMarker>,
    camera: Res<Camera>,
    mut tri_q: ResMut<TriQueue>,
) {
    let cam_id = positon_to_chunk_id(camera.eye.xz());
    // println!("Cam: {cam_id:?}");

    for mut id in tri_meshes.iter_mut() {
        let id = &mut (*id);
        if wrap_chunk(cam_id, &mut id.computed_chunk_id, id.base_chunk_id.clone()) {
            tri_q.queue.push_back(id.clone())
        }
    }
}

fn wrap_chunk(cam_id: IVec2, chunk_position: &mut IVec2, offset: IVec2) -> bool {
    let old = *chunk_position;
    let new = wrap(cam_id, offset, IVec2::splat(RENDER_DISTANCE as i32));

    *chunk_position = new;

    return new != old;
}

fn wrap(p: IVec2, offset: IVec2, period: IVec2) -> IVec2 {
    let wraped = (p - offset + period) / (period * 2);
    wraped * 2 * period + offset
}

fn init_meshes(mut tri_q: ResMut<TriQueue>, mut commands: Commands) {
    let mut i = 0;
    for x in -(RENDER_DISTANCE as i32)..RENDER_DISTANCE as i32 {
        for y in -(RENDER_DISTANCE as i32)..RENDER_DISTANCE as i32 {
            let marker = TriMeshMarker {
                offset: i,
                base_chunk_id: IVec2 { x, y },
                computed_chunk_id: IVec2 { x, y },
            };
            commands.spawn(marker);
            tri_q.queue.push_back(marker);
            i += 1;
        }
    }
}
