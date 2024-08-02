use std::ops::Deref;

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
    schedule::IntoSystemConfigs,
    system::{Commands, Local, Query, Res, ResMut},
};

use glam::{vec2, vec3, Quat, Vec2, Vec3};
use winit::event::VirtualKeyCode;

#[derive(Debug, Component)]
pub struct PlayerControllerUpright {
    pub rotation: Quat,
    pub pitch: f32,
    pub yaw: f32,
    pub position: Vec3,
    pub momentum: f32,
    pub speed: f32,
}

#[derive(Debug, Component)]
pub struct PlayerController {
    pub speed: f32,
    pub momentum: f32,
}

pub fn update<'a>(
    mut player: Query<(&mut PlayerController, &mut Transform)>,
    mut camera: ResMut<Camera>,
    time: Res<Time>,
    kb: Res<ButtonInput>,
    mut mouse_events: EventReader<MouseDelta>,
    axis: Res<AxisInput>,
    look_ahead: Local<Quat>,
    // bodies: Query<(&Physics, &Transform), Without<PlayerController>>,
) {
    let (mut player_controller, mut transform) = player.single_mut();
    let (player_controller, transform) = (&mut *player_controller, &mut *transform);

    // let gravity = gravity_all(transform.translation, bodies.iter()) * time.delta_seconds * -10.0;
    // transform.translation += gravity;
    // camera.eye += gravity;
    // camera.target += gravity;

    if kb.is_down(VirtualKeyCode::W) {
        player_controller.momentum += player_controller.speed * time.delta_seconds
    }
    if kb.is_down(VirtualKeyCode::S) {
        player_controller.momentum *= 0.95
    }
    let camera_displacement_factor = 0.04;
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
        + (transform.up() * 1.5 + transform.forward() * 2.0) * camera_displacement_factor;
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

pub fn update_camera_from_upright_player_controller(
    player_query: Query<&mut PlayerControllerUpright>,
    mut camera: ResMut<Camera>,
) {
    let player_controller = player_query.single();
    camera.fovy = std::f32::consts::PI * 0.25 * (2.0 - (-player_controller.momentum * 0.1).exp());

    camera.eye = player_controller.position;
    camera.target =
        player_controller.position + Transform::forward_of_quat(player_controller.rotation);
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

pub fn update_upright_player_controller<'a>(
    mut player: Query<(&mut PlayerControllerUpright, &mut Transform)>,
    time: Res<Time>,
    kb: Res<ButtonInput>,
    // average_forward: Local<>,
    // bodies: Query<(&Physics, &Transform), Without<PlayerController>>,
    mut roll_average: Local<f32>,
) {
    let (mut player_controller, mut ship_transform) = player.single_mut();
    let (player_controller, ship_transform) = (&mut *player_controller, &mut *ship_transform);

    // let gravity = gravity_all(transform.translation, bodies.iter()) * time.delta_seconds * -10.0;
    // transform.translation += gravity;
    // camera.eye += gravity;
    // camera.target += gravity;

    if kb.is_down(VirtualKeyCode::W) {
        player_controller.momentum += player_controller.speed * time.delta_seconds
    }
    if kb.is_down(VirtualKeyCode::S) {
        player_controller.momentum *= 0.95
    }
    let camera_displacement_factor = 0.04;

    let roll_target = Transform::left_of_quat(player_controller.rotation);

    // average_forward = average_forward

    let pc_forward = Transform::forward_of_quat(player_controller.rotation);
    let roll_ammount = pc_forward.dot(ship_transform.left()) * 4.0;
    *roll_average = lerp(
        *roll_average,
        roll_ammount,
        1.0 - (-time.delta_seconds * 10.0).exp(),
    );

    // ship_transform.rotation = full_rot;
    let ship_roll = (player_controller.rotation * Quat::from_rotation_z(*roll_average))
        * player_controller.rotation.inverse();

    ship_transform.rotation = ship_transform.rotation.lerp(
        ship_roll * player_controller.rotation,
        1.0 - (-time.delta_seconds * 5.0).exp(),
    );

    let frame_translation = player_controller.momentum
        * player_controller.speed
        * time.delta_seconds
        * ship_transform.forward()
        * 0.1;
    ship_transform.translation += frame_translation;
    player_controller.position = ship_transform.translation
        + ship_transform.back() * 0.2
        + ship_transform.up() * 0.05
        + ship_transform.right() * 0.02;

    // let x_rotation = (cam_transform.rotation.inverse()
    //     * Quat::from_rotation_x(mouse_sensitivity * (mouse_delta.y + axis.y) * time.delta_seconds))
    //     * cam_transform.rotation;

    // let y_rotation = (cam_transform.rotation.inverse()
    //     * Quat::from_rotation_y(mouse_sensitivity * (mouse_delta.x + axis.x) * time.delta_seconds))
    //     * cam_transform.rotation;

    // let body_rotation = x_rotation * y_rotation;

    // ship_transform.rotation = body_rotation * ship_transform.rotation;
    // look_ahead.lerp(
    //     body_rotation * 10.0,
    //     1.0 - (-1.0 * time.delta_seconds).exp(),
    // );

    // // Correct view
    // let camera_alginment_full_rotation = Quat::from_rotation_arc(camera.up, ship_transform.up());

    // let focal_ponit = ship_transform.translation
    //     + (ship_transform.up() * 1.5 + ship_transform.forward() * 2.0) * camera_displacement_factor;
    // let camera_eye_target = focal_ponit + ship_transform.back() * camera_displacement_factor * 6.0;
    // let camera_target_target = focal_ponit
    //     + look_ahead.mul_vec3(ship_transform.forward())
    //         * camera_displacement_factor
    //         * 3.0
    //         * (player_controller.momentum + 1.0).ln();

    // let lerp_factor = 1.0
    //     - (-time.delta_seconds
    //         * 020.0
    //         * (1.0 / (1.1 + camera_transform.forward().dot(ship_transform.forward())) + 1.0))
    //         .exp();
    // camera.eye += frame_translation;
    // camera.target += frame_translation;
    // camera.target = camera.target.lerp(camera_target_target, lerp_factor);
    // camera.eye = camera.eye.lerp(camera_eye_target, lerp_factor);
}

fn spawn_player(mut commands: Commands) {
    let spawn_position = vec3(0.0, 0.0, -200.0);

    commands.spawn((
        // PlayerController::new(10.1),
        PlayerControllerUpright {
            rotation: Quat::from_rotation_y(0.0),
            position: spawn_position,
            pitch: 0.0,
            yaw: 0.0,
            momentum: 0.0,
            speed: 10.0,
        },
        Transform {
            scale: Vec3::ONE * 0.01,
            translation: spawn_position,
            rotation: Default::default(),
        },
        // Physics {
        //     mass: 1.0,
        //     ..Default::default()
        // },
        Ship,
    ));
}

fn update_upright_pitch_and_yaw(
    mut mouse_events: EventReader<MouseDelta>,
    // axis: Res<AxisInput>,
    mut players: Query<&mut PlayerControllerUpright>,
    time: Res<Time>,
) {
    let mouse_sensitivity = 0.3;
    let mouse_delta = mouse_events
        .read()
        .map(|MouseDelta { x, y }| vec2(*x, *y))
        .sum::<Vec2>()
        * mouse_sensitivity
        * time.delta_seconds;

    for mut player in players.iter_mut() {
        player.pitch += mouse_delta.y;
        player.yaw += mouse_delta.x;
        let max_pitch = std::f32::consts::PI * 0.4;
        player.pitch = player.pitch.clamp(-max_pitch, max_pitch);
        player.rotation = Quat::from_euler(glam::EulerRot::YXZ, player.yaw, player.pitch, 0.0);
    }
}

pub fn init(ecs: &mut Ecs) {
    // ecs.world.spawn(());
    ecs.init.add_systems(spawn_player);
    // ecs.update.add_systems(update);
    ecs.update.add_systems(update_upright_pitch_and_yaw);
    ecs.update
        .add_systems(update_upright_player_controller.after(update_upright_pitch_and_yaw));
    ecs.update.add_systems(
        update_camera_from_upright_player_controller.after(update_upright_player_controller),
    );
    // ecs.update
    //     .add_systems(crate::gizmo::update_arrows.after(update));
}
