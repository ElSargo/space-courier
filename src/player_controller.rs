use crate::{
    camera::Camera,
    ecs::Initailiser,
    input::{
        AxisInput, ButtonInput,
        Inputs::{Back, Forward},
    },
    physics::Physics,
    ship::Ship,
    time::Time,
    transform::Transform,
};
use bevy_ecs::{
    component::Component,
    schedule::IntoSystemConfigs,
    system::{Commands, Local, Query, Res, ResMut},
};

use glam::{vec3, Quat, Vec3};

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
    pub _skating: bool,
}

#[allow(dead_code)]
pub fn update<'a>(
    mut player: Query<(&mut PlayerController, &mut Transform)>,
    mut camera: ResMut<Camera>,
    time: Res<Time>,
    kb: Res<ButtonInput>,
    axis: Res<AxisInput>,
    look_ahead: Local<Quat>,
) {
    let (mut player_controller, mut transform) = player.single_mut();
    let (player_controller, transform) = (&mut *player_controller, &mut *transform);

    // let gravity = gravity_all(transform.translation, bodies.iter()) * time.delta_seconds * -10.0;
    // transform.translation += gravity;
    // camera.eye += gravity;
    // camera.target += gravity;

    if kb.active(Forward) {
        player_controller.momentum += player_controller.speed * time.delta_seconds
    }
    if kb.active(crate::input::Inputs::Back) {
        player_controller.momentum *= 0.95
    }
    let camera_displacement_factor = 0.04;
    camera.fovy = std::f32::consts::PI * 0.25 * (2.0 - (-player_controller.momentum * 0.1).exp());

    let mouse_sensitivity = 0.3;

    let cam_transform = Transform::from_mat4(&camera.view_matrix());

    let x_rotation = (cam_transform.rotation.inverse()
        * Quat::from_rotation_x(mouse_sensitivity * (axis.y) * time.delta_seconds))
        * cam_transform.rotation;

    let y_rotation = (cam_transform.rotation.inverse()
        * Quat::from_rotation_y(mouse_sensitivity * (axis.x) * time.delta_seconds))
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
    mut cinematic_smoothing: Local<Quat>,
    time: Res<Time>,
) {
    let player_controller = player_query.single();
    camera.fovy =
        std::f32::consts::PI * 0.25 * (1.5 - 0.5 * (-player_controller.momentum * 0.1).exp());

    *cinematic_smoothing = cinematic_smoothing.lerp(
        player_controller.rotation,
        1.0 - (-5.0 * (player_controller.momentum + std::f32::consts::E).ln() * time.delta_seconds)
            .exp(),
    );

    camera.eye = player_controller.position;
    camera.target = player_controller.position + Transform::forward_of_quat(*cinematic_smoothing);
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

    if kb.active(Forward) {
        player_controller.momentum += player_controller.speed * time.delta_seconds
    }
    if kb.active(Back) {
        player_controller.momentum *= 0.95
    }

    let pc_forward = Transform::forward_of_quat(player_controller.rotation);
    let roll_ammount = pc_forward.dot(ship_transform.left()) * 4.0;
    *roll_average = lerp(
        *roll_average,
        roll_ammount,
        1.0 - (-time.delta_seconds * 10.0).exp(),
    );

    // ship_transform.rotation = full_rot;
    let ship_roll = (player_controller.rotation
        * Quat::from_rotation_z(*roll_average)
        * player_controller.rotation.inverse())
    .lerp(Quat::IDENTITY, (-0.2 * player_controller.momentum).exp());

    ship_transform.rotation = ship_transform.rotation.lerp(
        ship_roll * player_controller.rotation,
        1.0 - (-time.delta_seconds * 5.0).exp(),
    );

    let frame_translation = player_controller.momentum
        * player_controller.speed
        * time.delta_seconds
        * ship_transform.forward();
    ship_transform.translation += frame_translation;
    ship_transform.translation.y = 0.02;
    player_controller.position = ship_transform.translation
        + ship_transform.back() * 0.4
        + ship_transform.up() * 0.1
        + ship_transform.right() * 0.04;
}

fn spawn_player(mut commands: Commands) {
    let spawn_position = vec3(0.0, 0.0, 0.0);

    commands.spawn((
        // PlayerController::new(10.1),
        PlayerControllerUpright {
            rotation: Quat::from_rotation_y(0.0),
            position: spawn_position,
            pitch: 0.0,
            yaw: 0.0,
            momentum: 0.0,
            speed: 1.0,
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
    axis: Res<AxisInput>,
    mut players: Query<&mut PlayerControllerUpright>,
    time: Res<Time>,
) {
    let mouse_sensitivity = 0.3;

    for mut player in players.iter_mut() {
        player.pitch += axis.y * time.delta_seconds * mouse_sensitivity;
        player.yaw += axis.x * time.delta_seconds * mouse_sensitivity;
        let max_pitch = std::f32::consts::PI * 0.4;
        player.pitch = player.pitch.clamp(-max_pitch, max_pitch);
        player.rotation = Quat::from_euler(glam::EulerRot::YXZ, player.yaw, player.pitch, 0.0);
    }
}

pub fn init_mod(ecs: &mut Initailiser) {
    // ecs.world.spawn(());
    ecs.init.add_systems(spawn_player);
    // ecs.update.add_systems(update);
    ecs.update.add_systems((
        update_upright_pitch_and_yaw,
        apply_velocity,
        bank_player
            .before(update_upright_player_controller)
            .before(update_upright_pitch_and_yaw), // update_momentum_from_velocity,
        update_upright_player_controller.after(update_upright_pitch_and_yaw),
        update_camera_from_upright_player_controller.after(update_upright_player_controller),
    ));
    // ecs.update
    //     .add_systems();
    // ecs.update.add_systems();
    // ecs.update
    //     .add_systems(crate::gizmo::update_arrows.after(update));
}

pub fn apply_velocity(
    mut players: Query<(&mut PlayerControllerUpright, &Physics)>,
    time: Res<Time>,
) {
    for (mut pc, Physics { velocity, .. }) in players.iter_mut() {
        pc.position += *velocity * time.delta_seconds;
    }
}

pub fn bank_player(mut players: Query<(&mut PlayerControllerUpright, &mut Transform)>) {
    for (mut pc, mut tra) in players.iter_mut() {
        let tra = &mut *tra;
        if tra.translation.y < 0.0 {
            tra.translation.y = 0.0;
            pc.pitch = -pc.pitch.abs();
        }
    }
}
