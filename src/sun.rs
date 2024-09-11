use crate::shader_manager::UnifiedPipelineModuleResource;
use crate::DeviceRes;
use crate::QueueRes;
use crate::TextureBindGroupLayout;
use bevy_ecs::prelude::Res;
use bevy_ecs::system::Commands;
use bevy_ecs::system::In;
use std::sync::{Arc, Mutex};

use anymap::AnyMap;
use bevy_ecs::{
    component::Component,
    query::With,
    system::{Query, Resource},
};
use glam::Vec3;
use wgpu::Device;

use crate::{
    camera::{Camera, CameraUniform},
    ecs::Ecs,
    get_all,
    gpu::InstanceBuffer,
    instance_buffer_bytes, mk_pipeline, mk_pipeline_layout,
    model::Model,
    resources,
    shader_manager::{PipelineHandle, ShaderManagerCtrl, UnifiedPipelineModuleDescriptor},
    transform::Transform,
    RawInstance,
};

pub(crate) const SUN_MASS: f32 = 20000.0;

pub fn update_instance_buffer(
    camera: Res<Camera>,
    transforms: Query<&Transform, With<Sun>>,
    ib: Res<SunIB>,
    devicedevice Res<DeviceRes>,
    QueueRes(queue): Res<QueueRes>,
    // ship_data: ResMut<ShipGpuResources>,
) {
    let ship_bytes = instance_buffer_bytes(transforms.iter(), camera.eye);
    ib.0.lock()
        .unwrap()
        .write(&device, &ship_bytes.as_slice(), &queue);
}

#[derive(Resource)]
pub struct SunRenderReasources {
    pub sun_transform_instance_buffer: Arc<Mutex<InstanceBuffer>>,
    pub sun_render_pipeline: PipelineHandle,
    pub sun_model: Model,
}

#[derive(Resource)]
pub struct SunIB(Arc<Mutex<InstanceBuffer>>);

#[derive(Component)]
pub struct Sun;

pub fn init(
    ecs: &mut Ecs,
    // DeviceRes(device): Res<DeviceRes>,
    device: Res<DeviceRes>,
    QueueRes(queue): Res<QueueRes>,
    shader_manager: Res<ShaderManagerCtrl>,
    texture_bind_group_layout: Res<TextureBindGroupLayout>,
    cgs: Res<CameraUniform>,
    commands: Commands,
) {
    let sun_model = resources::load_model(
        "wallmart.obj",
        &device,
        &queue,
        &texture_bind_group_layout.0,
    )
    .block_on()
    .unwrap();

    // let sun_transforms = vec![];

    ecs.world.spawn((
        Sun,
        Transform {
            scale: Vec3::ONE * 20.0,
            ..Default::default()
        },
    ));

    // let sun_transform_buffer_data =
    //     instance_buffer_bytes_from_components(sun_transforms.iter().map(
    //         |Transform {
    //              scale,
    //              translation,
    //              rotation,
    //          }| (*translation, *rotation, *scale),
    //     ));
    let sun_transform_instance_buffer =
        Arc::new(Mutex::new(InstanceBuffer::new_init::<RawInstance>(
            Some("Sun instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &device,
            // bytemuck::cast_slice(&sun_transform_buffer_data),
            &[],
        )));

    let layout = {
        Box::leak(Box::new(mk_pipeline_layout(
            &device,
            "Sun",
            &[&cgs.inner.bind_group_layout, &texture_bind_group_layout.0],
        )))
    };

    ecs.world
        .insert_resource(SunIB(sun_transform_instance_buffer.clone()));
    commands.insert_resource({
        let sun_shader = shader_manager.load_shader("./src/sun.wgsl");

        SunRenderReasources {
            sun_transform_instance_buffer,
            sun_render_pipeline: shader_manager.add_render_pipeline(
                UnifiedPipelineModuleDescriptor { handle: sun_shader },
                |In(module): In<UnifiedPipelineModuleResource>, device: Res<DeviceRes>| {
                    mk_pipeline(&device, &layout, None, &module, "sun", RawInstance::desc())
                },
            ),
            sun_model,
        }
    });

    ecs.extract.add_systems(update_instance_buffer);
}
