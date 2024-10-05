#![allow(dead_code)]

use std::mem::size_of;

use bevy_ecs::{component::Component, query::With};
use glam::{vec3, vec4, Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};
use wgpu::VertexAttribute;

use crate::{
    ecs::Initailiser,
    gpu::{pack_buffers, Packable},
    instance_layout,
    transform::Transform,
    vertex_layout,
};

#[derive(Debug, Component)]
pub struct ArrowGizmo {
    pub color: Vec4,
}

// impl ArrowGizmos {
//     pub fn len(&self) -> usize {
//         self.transforms.len()
//     }

//     pub fn to_raw_buffer(&self) -> Vec<RawArrowInstance> {
//         self.transforms
//             .iter()
//             .zip(self.colors.iter())
//             .map(|(transform, color)| RawArrowInstance {
//                 transform: transform.to_mat(),
//                 color: *color,
//             })
//             .collect()
//     }
// }

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
pub fn init(ecs: &mut Initailiser) {
    for color in [
        vec4(1.0, 0.5, 0.25, 1.0),
        vec4(0.2, 0.5, 1.0, 1.0),
        vec4(0.2, 1., 0.5, 1.0),
    ] {
        ecs.world.spawn((
            Transform {
                scale: Vec3::ONE * 0.01,
                ..Default::default()
            },
            ArrowGizmo { color },
        ));
    }

    ecs.prepare_buffers
        .add_systems(pack_buffers::<RawArrowInstance, (), With<ArrowBufferMarker>>);
    // ecs.update.add_systems(update_arrows);
}

impl Packable for RawArrowInstance {
    type Input<'a> = (&'a Transform, &'a ArrowGizmo);

    fn pack<'a>(input: Self::Input<'a>, eye: Vec2) -> Self {
        Self {
            transform: input.0.translated(-eye.extend(0.0).xzy()).to_mat(),
            color: input.1.color,
        }
    }
}

// {
//             let arrow_instances =
//                 QueryBuilder::<(&Transform, &ArrowGizmo)>::new(&mut self.ecs.world)
//                     .build()
//                     .iter(&self.ecs.world)
//                     .map(|(transform, gizmo)| RawArrowInstance {
//                         transform: transform.to_mat(),
//                         color: gizmo.color,
//                     })
//                     .collect::<Vec<_>>();

//             self.arrow_transform_instance_buffer.write(
//                 &self.gpu.device,
//                 &arrow_instances.as_slice(),
//                 &self.gpu.queue,
//             );
//         }
