use crate::gpu::{pack_buffers, Packable};
use crate::transform::Transform;
use crate::{ecs::Initailiser, gpu::RD};
use crate::{
    instance_buffer,
    renderables::{SolidDrawable, SolidRenderable},
    shader_manager::{ShaderManagerCtrl, SolidPipelineModuleResourceExtras, SolidShader},
};
use crate::{instance_layout, vertex_layout};
use bevy_ecs::component::Component;
use bevy_ecs::query::With;
use bevy_ecs::system::{Commands, Res};
use bytemuck::cast_slice;
use glam::{vec3, Mat4, Quat, Vec3, Vec3Swizzles};
use rand::{thread_rng, Rng};
use std::{any::Any, sync::Arc};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BufferUsages, IndexFormat,
};

const SQRT_3_FRAC_2: f32 = 0.8660254;
const SQRT_3_FRAC_4: f32 = 0.4330127;
const VERTS: [Vec3; 4] = [
    vec3(0.0, 1.0, 0.0),
    vec3(-SQRT_3_FRAC_2, -0.5, 0.0),
    vec3(SQRT_3_FRAC_4, -0.5, 0.75),
    vec3(SQRT_3_FRAC_4, -0.5, -0.75),
];
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Vertex {
    position: [f32; 3],
    color: f32,
}
vertex_layout!(Vertex,[0 => Float32x3, 1 => Float32] );

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Zeroable, bytemuck::Pod)]
pub struct Instance {
    mat: Mat4,
}
instance_layout!(Instance,[ 2 => Float32x4  ,3 => Float32x4,  4 => Float32x4,  5 => Float32x4 ]);

impl Packable for Instance {
    type Input<'a> = &'a Transform;

    fn pack<'a>(
        intput: <<Self::Input<'a> as bevy_ecs::query::QueryData>::ReadOnly as bevy_ecs::query::WorldQuery>::Item<'_>,
        eye: glam::Vec2,
    ) -> Self {
        Self {
            mat: intput.translated(-eye.extend(0.0).xzy()).to_mat(),
        }
    }
}

const BATCH_SIZE: usize = 3_000;
fn init(device: RD, mut commands: Commands, shader_manager: Res<ShaderManagerCtrl>) {
    let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Stars batch vertex buffer"),
        usage: BufferUsages::VERTEX,
        contents: cast_slice(make_vertex_data(BATCH_SIZE).as_slice()),
    });

    let index_data: Vec<u16> = (0..BATCH_SIZE)
        .flat_map(|idx| {
            let base = idx as u16 * 4;
            [
                2, 1, 0, //
                1, 2, 3, //
                2, 0, 3, //
                1, 3, 0,
            ]
            .iter()
            .rev()
            .map(move |x| x + base)
        })
        .collect();

    let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Stars batch index buffer"),
        usage: BufferUsages::INDEX,
        contents: cast_slice(index_data.as_slice()),
    });

    let pipeline = shader_manager.add_render_pipeline(
        SolidShader {
            handle: shader_manager.load_shader("./src/stars.wgsl"),
        },
        || SolidPipelineModuleResourceExtras {
            vertex_buffers: Arc::new([Vertex::desc(), Instance::desc()]),
            label: Some("Stars pipeline"),
            ..Default::default()
        },
    );

    println!(
        "Verts {:?}",
        vertex_buffer.size() as usize / std::mem::size_of::<Vertex>()
    );

    commands.spawn((
        SolidRenderable {
            pipeline,
            renderable: Box::new(Stars {
                vertex_buffer,
                index_buffer,
            }),
        },
        instance_buffer!(@ "Star instance buffer", &device),
        Star,
    ));
}

fn spawn_stars(mut commands: Commands) {
    commands.spawn((
        Transform {
            scale: Vec3::splat(400.0),
            translation: vec3(-100.0, 2.0, -100.0),
            ..Default::default()
        },
        Star,
    ));
}

pub fn init_mod(initailiser: &mut Initailiser) {
    initailiser.init.add_systems(init);
    initailiser.init.add_systems(spawn_stars);
    initailiser
        .prepare_buffers
        .add_systems(pack_buffers::<Instance, With<Star>, With<Star>>);
}

#[derive(Component)]
struct Star;

fn make_vertex_data(batch_size: usize) -> Vec<Vertex> {
    let batch_bounds = vec3(1.0, 0.5, 1.0);
    let scale_bounds = 0.001..0.003;
    let mut rng = thread_rng();

    let mut batch = Vec::with_capacity(batch_size * 4);
    for _ in 0..batch_size {
        let position = rng.gen::<Vec3>() * batch_bounds;

        let rotation: Quat = rng.gen();
        let scale = rng.gen_range(scale_bounds.clone());

        let color = rng.gen();
        for vert in VERTS {
            batch.push(Vertex {
                position: (rotation.mul_vec3(vert) * scale + position).to_array(),
                color,
            })
        }
    }

    batch
}

#[derive(Debug)]
struct Stars {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
}

impl SolidDrawable for Stars {
    type Resource = ();
    fn draw<'a: 'b, 'b>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'b>,
        instances: std::ops::Range<u32>,
        _asset: &'a Self::Resource,
    ) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
        render_pass.draw_indexed(0..BATCH_SIZE as u32 * 12, 0, instances);
    }

    fn aquire(&self) -> Option<Arc<Self::Resource>> {
        Some(Arc::new(()))
    }
}
