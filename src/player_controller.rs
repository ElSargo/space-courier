use crate::{
    camera::Camera,
    ecs::Ecs,
    input::{AxisInput, ButtonInput, MouseDelta},
    time::Time,
    transform::Transform,
    Ship,
};
use bevy_ecs::{
    component::Component,
    event::EventReader,
    system::{Commands, Local, Query, Res, ResMut},
};

use glam::{vec2, vec3, Quat, Vec2, Vec3};
use winit::event::VirtualKeyCode;

#[derive(Debug, Component)]
pub struct PlayerController {
    pub speed: f32,
    pub momentum: f32,
}

impl PlayerController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            momentum: 0.0,
        }
    }
}

pub fn update<'a>(
    mut player: Query<(&mut PlayerController, &mut Transform)>,
    mut camera: ResMut<Camera>,
    time: Res<Time>,
    kb: Res<ButtonInput>,
    mut mouse_events: EventReader<MouseDelta>,
    axis: Res<AxisInput>,
    look_ahead: Local<Quat>,
) {
    let (mut player_controller, mut transform) = player.single_mut();
    let (player_controller, transform) = (&mut *player_controller, &mut *transform);

    if kb.is_down(VirtualKeyCode::W) {
        player_controller.momentum += player_controller.speed * time.delta_seconds
    }
    if kb.is_down(VirtualKeyCode::S) {
        player_controller.momentum *= 0.95
    }
    let camera_displacement_factor = 0.03;
    camera.fovy = std::f32::consts::PI * 0.25 * (2.0 - (-player_controller.momentum * 0.1).exp());
    let mouse_delta: Vec2 = mouse_events
        .read()
        .map(|MouseDelta { x, y }| vec2(*x, *y))
        .sum();

    let mouse_sensitivity = 0.3;

    let cam_transform = Transform::from_mat4(&camera.view_matrix());

    let x_rotation = (cam_transform.rotation.inverse()
        * Quat::from_rotation_x(mouse_sensitivity * (mouse_delta.y + axis.y) * time.delta_seconds))
        * cam_transform.rotation;

    let y_rotation = (cam_transform.rotation.inverse()
        * Quat::from_rotation_y(mouse_sensitivity * (mouse_delta.x + axis.x) * time.delta_seconds))
        * cam_transform.rotation;

    let body_rotation = x_rotation * y_rotation;

    transform.rotation = body_rotation * transform.rotation;
    look_ahead.lerp(
        body_rotation * 10.0,
        1.0 - (-1.0 * time.delta_seconds).exp(),
    );

    // Correct view
    let camera_alginment_full_rotation = Quat::from_rotation_arc(camera.up, transform.up());

    camera.up = Quat::IDENTITY
        .lerp(
            camera_alginment_full_rotation,
            1.0 - (-1.0 * time.delta_seconds).exp(),
        )
        .mul_vec3(camera.up);

    let frame_translation = player_controller.momentum
        * player_controller.speed
        * time.delta_seconds
        * transform.forward();
    transform.translation += frame_translation;

    let focal_ponit = transform.translation
        + (transform.up() + transform.forward() * 2.0) * camera_displacement_factor;
    let camera_eye_target = focal_ponit + transform.back() * camera_displacement_factor * 6.0;
    let camera_target_target = focal_ponit
        + look_ahead.mul_vec3(transform.forward())
            * camera_displacement_factor
            * 3.0
            * (player_controller.momentum + 1.0).ln();

    let lerp_factor = 1.0
        - (-time.delta_seconds
            * 020.0
            * (1.0 / (1.1 + cam_transform.forward().dot(transform.forward())) + 1.0))
            .exp();
    camera.eye += frame_translation;
    camera.target += frame_translation;
    camera.target = camera.target.lerp(camera_target_target, lerp_factor);
    camera.eye = camera.eye.lerp(camera_eye_target, lerp_factor);
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        PlayerController::new(10.1),
        Transform {
            scale: Vec3::ONE * 0.01,
            translation: vec3(0., 0., -200.),
            rotation: Default::default(),
        },
        Ship,
    ));
}

pub fn init(ecs: &mut Ecs) {
    // ecs.world.spawn(());
    ecs.init.add_systems(spawn_player);
    ecs.update.add_systems(update);
    // ecs.update
    //     .add_systems(crate::gizmo::update_arrows.after(update));
}
