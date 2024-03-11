use std::mem::size_of;

use bevy_ecs::{
    component::Component,
    query::{With, Without},
    system::{Query, Res},
};
use glam::{vec4, Mat4, Vec3, Vec4};

use crate::{camera::Camera, ecs::Ecs, player_controller::PlayerController, transform::Transform};

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
#[derive(Debug, Copy, Clone)]
pub struct RawArrowInstance {
    pub transform: Mat4,
    pub color: Vec4,
}
unsafe impl bytemuck::Pod for RawArrowInstance {}
unsafe impl bytemuck::Zeroable for RawArrowInstance {}

impl RawArrowInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 5,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                },
            ],
        }
    }
}

pub fn update_arrows(
    player: Query<(&PlayerController, &Transform), Without<ArrowGizmo>>,
    mut arrows: Query<&mut Transform, With<ArrowGizmo>>,
    camera: Res<Camera>,
) {
    let (player_controller, ship_transform) = player.single();
    let vel_dir = player_controller.velocity.normalize_or_zero();
    let mut arrows = arrows.iter_mut();
    let mut arrow_transform = arrows.next().unwrap();
    arrow_transform.translation = ship_transform.translation + vel_dir * 0.3;
    arrow_transform.look_to(vel_dir, camera.up);

    let mut arrow_transform = arrows.next().unwrap();
    let acc_dir = player_controller.debug_acceleration.normalize_or_zero();
    arrow_transform.translation = ship_transform.translation + acc_dir * 0.3;
    arrow_transform.look_to(acc_dir, camera.up);

    let mut arrow_transform = arrows.next().unwrap();
    arrow_transform.translation = player_controller.camera_target_debug;
    arrow_transform.look_at(ship_transform.translation, camera.up);
}

pub fn init(ecs: &mut Ecs) {
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
    // ecs.update.add_systems(update_arrows);
}
