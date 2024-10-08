use crate::gpu;
use crate::gpu::RD;
use crate::gpu::RQ;
use crate::gpu::RTBL;
use crate::model_loader;

use crate::physics::Physics;
use crate::shader_manager::Marker;
use crate::shader_manager::UnifiedPipelineModuleResource;
use crate::QueueResource;
use bevy_ecs::prelude::Res;
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::system::Commands;
use bevy_ecs::system::In;

use glam::Quat;

use bevy_ecs::{
    component::Component,
    query::With,
    system::{Query, Resource},
};
use glam::Vec3;

use crate::{
    camera::{Camera, CameraUniform},
    ecs::Initailiser,
    instance_buffer::InstanceBuffer,
    instance_buffer_bytes, mk_pipeline, mk_pipeline_layout,
    shader_manager::ShaderManagerCtrl,
    transform::Transform,
    RawInstance,
};

pub(crate) const SUN_MASS: f32 = 20000.0;

pub fn update_instance_buffer(
    camera: Res<Camera>,
    transforms: Query<&Transform, With<Sun>>,
    ib: Query<&InstanceBuffer, With<Sun>>,
    device: RD,
    queue: Res<QueueResource>,
    // ship_data: ResMut<ShipGpuResources>,
) {
    let ship_bytes = instance_buffer_bytes(transforms.iter(), camera.eye);
    ib.single().write(&device, &ship_bytes.as_slice(), &queue);
}

#[derive(Component)]
pub struct Sun;

pub fn init(
    device: RD,
    queue: RQ,
    shader_manager: Res<ShaderManagerCtrl>,
    texture_bind_group_layout: RTBL,
    cgs: Res<CameraUniform>,
    mut commands: Commands,
) {
    // device: Res<DeviceRes>,
    {
        let sun_model = model_loader::load_model(
            "wallmart.obj",
            device.0.clone(),
            queue.0.clone(),
            texture_bind_group_layout.0.clone(),
        );
        // .unwrap();

        // let sun_transforms = vec![];

        commands.spawn((
            Sun,
            Transform {
                scale: Vec3::ONE * 20.0,
                ..Default::default()
            },
        ));

        let sun_transform_instance_buffer = InstanceBuffer::new_init::<RawInstance>(
            Some("Sun instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &device,
            // bytemuck::cast_slice(&sun_transform_buffer_data),
            &[],
        );

        commands.insert_resource::<Marker<wgpu::PipelineLayout, SunPipelineLayout>>(Marker::new(
            mk_pipeline_layout(
                &device,
                "Sun",
                &[&cgs.bind_group_layout, &texture_bind_group_layout.0],
            ),
        ));

        let sun_shader = shader_manager.load_shader("./src/sun.wgsl");
        // commands.spawn((
        //     SolidRenderable {
        //         pipeline_handle: shader_manager.add_render_pipeline(
        //             UnifiedPipelineModuleDescriptor { handle: sun_shader },
        //             pipeline_constructor,
        //         ),
        //         model: sun_model,
        //     },
        //     Sun,
        //     sun_transform_instance_buffer,
        // ));
    }
}

fn spawn_sun(mut commands: Commands) {
    commands.spawn((
        Transform::default(),
        Physics {
            velocity: Vec3::ZERO,
            angular_velocity: Quat::IDENTITY,
            mass: 10e10,
        },
        Sun,
    ));
}

#[derive(Resource)]
struct SunPipelineLayout;

fn pipeline_constructor(
    In(module): In<UnifiedPipelineModuleResource>,
    layout: Res<Marker<wgpu::PipelineLayout, SunPipelineLayout>>,
    device: RD,
) -> wgpu::RenderPipeline {
    mk_pipeline(&device, &layout, None, &module, "sun", RawInstance::desc())
}

pub fn init_mod(ecs: &mut Initailiser) {
    // let pipeline_constructor = ecs.world.register_system(pipeline_constructor);

    ecs.init.add_systems((
        // spawn_sun,
        init.after(gpu::mk_texture_bind_group_layout_resource),
    ));

    // ecs.prepare_buffers.add_systems(update_instance_buffer);
}
