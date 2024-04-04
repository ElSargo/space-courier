use crate::{
    camera::Camera,
    ecs::Ecs,
    input::{AxisInput, ButtonInput},
    physics::{self, Physics},
    time::Time,
    transform::Transform,
    Ship,
};
use bevy_ecs::{
    component::Component,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut},
};

use glam::{vec3, Quat, Vec3};
use winit::event::VirtualKeyCode;

#[derive(Debug, Component)]
pub struct PlayerController {
    pub speed: f32,
    pub acceleration: Vec3,
    pub rotational_velocity: Vec3,
    pub rotated_by_mouse: bool,

    pub debug_acceleration: Vec3,
    pub camera_target_debug: Vec3,
}

impl PlayerController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,

            rotated_by_mouse: false,
            acceleration: Default::default(),
            rotational_velocity: Default::default(),
            camera_target_debug: Default::default(),
            debug_acceleration: std::default::Default::default(),
        }
    }

    pub fn mouse_motion(&mut self, delta: (f64, f64)) {
        self.rotated_by_mouse = true;
        self.rotational_velocity = vec3(delta.1 as f32 / 10.0, delta.0 as f32 / 10.0, 0.0);
    }
}

pub fn update_rotational_velocity(
    mut player_controller: Query<&mut PlayerController>,
    axis: Res<AxisInput>,
) {
    let mut pc = player_controller.single_mut();
    if !pc.rotated_by_mouse {
        pc.rotational_velocity.z = -axis.x;
        pc.rotational_velocity.x = axis.y;
        pc.rotational_velocity.y = axis.z;
    }
}

pub fn update<'a>(
    mut player: Query<(&mut PlayerController, &mut Transform, &mut Physics)>,
    mut camera: ResMut<Camera>,
    time: Res<Time>,
    kb: Res<ButtonInput>,
) {
    let (mut player_controller, mut ship_transform, mut phys) = player.single_mut();
    let (player_controller, transform) = (&mut *player_controller, &mut *ship_transform);
    player_controller.acceleration = Vec3::ZERO;

    if kb.is_down(VirtualKeyCode::W) {
        player_controller.acceleration += transform.forward() * player_controller.speed
    }
    if kb.is_down(VirtualKeyCode::S) {
        player_controller.acceleration += transform.back() * player_controller.speed
    }
    if kb.is_down(VirtualKeyCode::A) {
        player_controller.acceleration += transform.left() * player_controller.speed
    }
    if kb.is_down(VirtualKeyCode::D) {
        player_controller.acceleration += transform.right() * player_controller.speed
    }
    if kb.is_down(VirtualKeyCode::E) {
        player_controller.acceleration += transform.up() * player_controller.speed
    }
    if kb.is_down(VirtualKeyCode::Q) {
        player_controller.acceleration += transform.down() * player_controller.speed
    }
    player_controller.debug_acceleration = player_controller.acceleration;
    // player_controller.acceleration += gravity(ship_transform.translation) * crate::SUN_MASS;

    phys.velocity += player_controller.acceleration * time.delta_seconds;
    phys.velocity *= 1.0 - time.delta_seconds * 0.1;
    // transform.translation += player_controller.velocity * time.delta_seconds;
    camera.eye += phys.velocity * time.delta_seconds;
    transform.rotation = transform.rotation
        * Quat::from_euler(
            glam::EulerRot::XYZ,
            player_controller.rotational_velocity.x * 3.0 * time.delta_seconds,
            player_controller.rotational_velocity.y * 3.0 * time.delta_seconds,
            player_controller.rotational_velocity.z * 3.0 * time.delta_seconds,
        );
    if player_controller.rotated_by_mouse {
        player_controller.rotational_velocity = Vec3::ZERO;
        player_controller.rotated_by_mouse = false;
    }

    let velocity_magnitude = phys.velocity.length();
    let target_distance_coef = 0.001 * velocity_magnitude / (1.0 + velocity_magnitude * 0.001);
    let camera_eye_target = transform.translation
        + transform
            .rotation
            .mul_vec3(vec3(0.1, 0.2, -1. * (target_distance_coef * 5.0 + 0.5)));

    let speed = 10.0;

    let camera_eye_displacement = camera_eye_target - camera.eye;

    camera.eye += camera_eye_displacement * (time.delta_seconds * speed).min(1.0);

    let target = transform.translation + transform.forward() * (target_distance_coef * 20. + 1.0);
    camera.target = target;
    player_controller.camera_target_debug = target;
    camera.up = transform.up();
}

fn spawn_player(mut commands: Commands) {
    commands.spawn((
        PlayerController::new(100.0),
        Physics {
            velocity: Default::default(),
            angular_velocity: Default::default(),
            mass: 1.0,
        },
        Transform {
            scale: Vec3::ONE * 0.1,
            translation: vec3(0., 0., -200.),
            rotation: Default::default(),
        },
        Ship,
    ));
}

pub fn init(ecs: &mut Ecs) {
    // ecs.world.spawn(());
    ecs.init.add_systems(spawn_player);
    ecs.update
        .add_systems(update_rotational_velocity.before(update));
    ecs.update
        .add_systems(update.after(physics::apply_velocity));
    ecs.update
        .add_systems(crate::gizmo::update_arrows.after(update));
}
