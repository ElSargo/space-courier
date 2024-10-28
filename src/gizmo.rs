#![allow(dead_code)]

use crate::instance_buffer;
use crate::RQ;
use crate::{
    ecs::Initailiser,
    gpu::{pack_buffers, Packable, TextureBindGroupLayoutResource},
    instance_layout, model,
    model_loader::{self},
    transform::Transform,
    ShaderLoader, SolidPipelineModuleResourceExtras, SolidRenderable, SolidShader, RD,
};
use bevy_ecs::schedule::IntoSystemConfigs;
use bevy_ecs::{
    component::Component,
    query::With,
    system::{Commands, Res},
};
use glam::vec3;
use glam::{vec4, Mat4, Vec2, Vec3, Vec4};

#[derive(Debug, Component)]
pub struct ArrowGizmo {
    pub color: Vec4,
}

#[repr(C)]
#[derive(Component, Debug, Copy, Clone)]
pub struct RawArrowInstance {
    pub transform: Mat4,
    pub color: Vec4,
}
unsafe impl bytemuck::Pod for RawArrowInstance {}
unsafe impl bytemuck::Zeroable for RawArrowInstance {}

instance_layout!(RawArrowInstance, [ 5 => Float32x4, 6 => Float32x4, 7 => Float32x4, 8 => Float32x4, 9 => Float32x4, ]);

// pub fn update_arrows(
//     player: Query<(&PlayerController, &Transform, &Physics), Without<ArrowGizmo>>,
//     mut arrows: Query<&mut Transform, With<ArrowGizmo>>,
//     camera: Res<Camera>,
// ) {
//     let (player_controller, ship_transform, phys) = player.single();
//     let vel_dir = phys.velocity.normalize_or_zero();
//     let mut arrows = arrows.iter_mut();
//     let mut arrow_transform = arrows.next().unwrap();
//     arrow_transform.translation = ship_transform.translation + vel_dir * 0.3;
//     arrow_transform.look_to(vel_dir, camera.up);

//     let mut arrow_transform = arrows.next().unwrap();
//     let acc_dir = player_controller.debug_acceleration.normalize_or_zero();
//     arrow_transform.translation = ship_transform.translation + acc_dir * 0.3;
//     arrow_transform.look_to(acc_dir, camera.up);

//     let mut arrow_transform = arrows.next().unwrap();
//     arrow_transform.translation = player_controller.camera_target_debug;
//     arrow_transform.look_at(ship_transform.translation, camera.up);
// }

#[derive(Component)]
pub struct ArrowBufferMarker;
pub fn init_mod(ecs: &mut Initailiser) {
    // for color in [
    //     vec4(1.0, 0.5, 0.25, 1.0),
    //     vec4(0.2, 0.5, 1.0, 1.0),
    //     vec4(0.2, 1., 0.5, 1.0),
    // ] {
    //     ecs.world.spawn((
    //         Transform {
    //             scale: Vec3::ONE * 0.1,
    //             ..Default::default()
    //         },
    //         ArrowGizmo { color },
    //     ));
    // }

    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                let coord = vec3(x as f32, y as f32, z as f32);
                ecs.world.spawn((
                    Transform {
                        scale: Vec3::ONE * 0.1,
                        translation: coord,
                        ..Default::default()
                    },
                    ArrowGizmo {
                        color: coord.extend(1.0),
                    },
                ));
            }
        }
    }

    ecs.init.add_systems(
        init.after(crate::background::init_background)
            .after(crate::solid_pass::init_normal_pipeline),
    );
    // // .after(crate::background::init_background)
    // .after(crate::solid_pass::init_normal_pipeline),));
    ecs.prepare_buffers
        .add_systems(pack_buffers::<RawArrowInstance, (), With<ArrowBufferMarker>>);
    // ecs.update.add_systems(update_arrows);
}

fn init(
    device: RD,
    queue: RQ,
    mut commands: Commands,
    sl: Res<ShaderLoader>,
    tr: Res<TextureBindGroupLayoutResource>,
) {
    let shader = sl.load_shader("./src/gizmo.wgsl");

    let pipeline = sl.add_render_pipeline(SolidShader { handle: shader }, move || {
        SolidPipelineModuleResourceExtras {
            vertex_buffers: [model::ModelVertex::desc(), RawArrowInstance::desc()].into(),
            label: Some("Gizmo pipeline"),
            ..Default::default()
        }
    });

    commands.spawn((
        SolidRenderable {
            pipeline,
            renderable: Box::new(model_loader::load_model(
                "arrow.obj",
                device.0.clone(),
                queue.0.clone(),
                tr.0.clone(),
            )),
        },
        instance_buffer!(@ "Gizmo",& device),
        ArrowBufferMarker,
    ));
}

// impl SolidDrawable  for
// ArrowBufferMarker {
//     type Resource = ();

//     fn draw<'a: 'b, 'b>(
//         &self,
//         render_pass: &mut wgpu::RenderPass,
//         instances: std::ops::Range<u32>,
//         _asset: &'a Self::Resource,
//     ) {
//         // TODO set instance buffer properly
//         // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
//         render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
//         render_pass.set_push_constants(ShaderStages::VERTEX, 0, &(CHUNK_DIM as u32).to_ne_bytes());
//         render_pass.draw_indexed(0..self.index_buffer.size() as u32 / 2, 0, instances);
//     }

//     fn aquire(&self) -> Option<Arc<Self::Resource>> {
//         Some(Arc::new(()))
//     }
// }
impl Packable for RawArrowInstance {
    type Input<'a> = (&'a Transform, &'a ArrowGizmo);

    fn pack<'a>(input: Self::Input<'a>, eye: Vec2, _eye_forward: Vec2) -> Option<Self> {
        Some(Self {
            transform: input.0.translated(-eye.extend(0.0)).to_mat(),
            color: input.1.color,
        })
    }
}
