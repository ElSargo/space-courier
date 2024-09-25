use bevy_ecs::{
    component::Component,
    query::With,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res},
};

use crate::{
    background,
    camera::Camera,
    ecs::Initailiser,
    gpu::{
        pack_buffers, InstanceBuffer, QueueResource, SolidRenderable,
        TextureBindGroupLayoutResource, RD, RQ,
    },
    instance_buffer_bytes, model_loader,
    transform::Transform,
    RawInstance, StandardRenderPipeline,
};

#[derive(Component)]
pub struct Ship;

pub fn init_ship(
    device: RD,
    mut commands: Commands,
    tr: Res<TextureBindGroupLayoutResource>,
    pipeline: Res<StandardRenderPipeline>,
    queue: RQ,
) {
    // let sgr = ShipGpuResources {
    //     instance_buffer: Arc::new(Mutex::new(InstanceBuffer::new(
    //         Some("Ship instance Buffer".to_owned()),
    //         wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    //         &device,
    //     ))),
    // };
    // commands.insert_resource(sgr.clone());
    let ship_model = model_loader::load_model(
        "retroShip.obj",
        device.0.clone(),
        queue.0.clone(),
        tr.0.clone(),
    );

    commands.spawn((
        SolidRenderable {
            model: ship_model,
            pipeline_handle: pipeline.handle,
        },
        Ship,
        InstanceBuffer::new(
            Some("Ship instance Buffer".to_owned()),
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            &device,
        ),
    ));

    // let planet_transform_instance_buffer = Arc::new(Mutex::new(InstanceBuffer::new(
    //     Some("Planet instance buffer".to_owned()),
    //     BufferUsages::VERTEX | BufferUsages::COPY_DST,
    //     &device,
    // )));
    // // let ship_transform_instance_buffer = ;
}

pub fn update_instance_buffer(
    camera: Res<Camera>,
    transforms: Query<&Transform, With<Ship>>,
    device: RD,
    queue: Res<QueueResource>,
    ship_data: Query<&InstanceBuffer, With<Ship>>,
) {
    // let ship_bytes = instance_buffer_bytes(transforms.iter(), camera.eye);
    // ship_data
    //     .single()
    //     .write(&device, &ship_bytes.as_slice(), &queue);

    // ship_data.single().write(
    //     &device,
    //     &transforms
    //         .iter()
    //         .map(|t| t.translated(-camera.eye))
    //         .map(|t| RawInstance::from_transform(&t))
    //         .collect::<Vec<_>>(),
    //     &queue,
    // );
    ship_data.single().write_from_iterator(
        transforms
            .iter()
            .map(|t| t.translated(-camera.eye))
            .map(|t| RawInstance::from_transform(&t)),
        &device,
        &queue,
    );
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.prepare_buffers
        .add_systems(pack_buffers::<&Transform, RawInstance, With<Ship>, With<Ship>>);
    // ecs.prepare_buffers.add_systems(update_instance_buffer);
    ecs.init.add_systems(
        init_ship
            .after(background::init_background)
            .after(crate::solid_pass::init_normal_pipeline),
    );
}
