use gilrs::{Event, Gilrs};
use glam::{vec3, Quat, Vec3};
use winit::{dpi::PhysicalSize, event::VirtualKeyCode};
pub const G: f32 = -10.0;
use crate::{camera::Camera, transform::Transform, SUN_MASS};

#[derive(Debug)]
pub struct PlayerController {
    pub speed: f32,
    pub acceleration: Vec3,
    pub velocity: Vec3,
    pub rotational_velocity: Vec3,
    pub rotated_by_mouse: bool,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
    pub is_down_pressed: bool,
    pub is_up_pressed: bool,
    pub gilrs: Gilrs,

    pub camera_target_debug: Vec3,
}

impl PlayerController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            gilrs: Gilrs::new().unwrap(),
            velocity: Default::default(),
            rotated_by_mouse: false,
            acceleration: Default::default(),
            rotational_velocity: Default::default(),
            is_forward_pressed: Default::default(),
            is_backward_pressed: Default::default(),
            is_left_pressed: Default::default(),
            is_right_pressed: Default::default(),
            is_down_pressed: Default::default(),
            is_up_pressed: Default::default(),
            camera_target_debug: Default::default(),
        }
    }

    pub fn process_keys(&mut self, key: &VirtualKeyCode, pressed: bool) -> bool {
        match key {
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                self.is_forward_pressed = pressed;
                true
            }
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.is_left_pressed = pressed;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                self.is_backward_pressed = pressed;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.is_right_pressed = pressed;
                true
            }
            VirtualKeyCode::Q => {
                self.is_down_pressed = pressed;
                true
            }
            VirtualKeyCode::E => {
                self.is_up_pressed = pressed;
                true
            }
            _ => false,
        }
    }

    pub fn update<'a>(
        &mut self,
        camera: &mut Camera,
        ship: &mut Transform,
        delta: f32,
        planets: impl Iterator<Item = (&'a Vec3, &'a f32)>,
    ) {
        self.acceleration = Vec3::ZERO;

        if self.is_forward_pressed {
            self.acceleration += ship.forward() * self.speed
        }
        if self.is_backward_pressed {
            self.acceleration += ship.back() * self.speed
        }
        if self.is_left_pressed {
            self.acceleration += ship.left() * self.speed
        }
        if self.is_right_pressed {
            self.acceleration += ship.right() * self.speed
        }
        if self.is_up_pressed {
            self.acceleration += ship.up() * self.speed
        }
        if self.is_down_pressed {
            self.acceleration += ship.down() * self.speed
        }
        let prev_vel = self.velocity.normalize_or_zero();
        for (translation, mass) in planets {
            self.acceleration += gravity(camera.eye - *translation) * *mass;
        }
        // self.acceleration += gravity(ship.translation) * SUN_MASS;

        self.velocity += self.acceleration * delta;
        self.velocity *= 1.0 - delta * 0.1;
        ship.translation += self.velocity * delta;
        camera.eye += self.velocity * delta;
        ship.rotation = ship.rotation
            * Quat::from_euler(
                glam::EulerRot::XYZ,
                self.rotational_velocity.x * 3.0 * delta,
                self.rotational_velocity.y * 3.0 * delta,
                self.rotational_velocity.z * 3.0 * delta,
            );
        if self.rotated_by_mouse {
            self.rotational_velocity = Vec3::ZERO;
            self.rotated_by_mouse = false;
        }

        let velocity_magnitude = self.velocity.length();
        let target_distance_coef = 0.001 * velocity_magnitude / (1.0 + velocity_magnitude * 0.001);
        let camera_eye_target = ship.translation
            + ship
                .rotation
                .mul_vec3(vec3(0.1, 0.2, -1. * (target_distance_coef * 10.0 + 1.0)));

        let speed = 10.0;

        let camera_eye_displacement = camera_eye_target - camera.eye; // + (delta * self.velocity).powf(2.0) * 0.5;

        camera.eye += camera_eye_displacement * (delta * speed).min(1.0);

        let target = ship.translation + ship.forward() * (target_distance_coef * 20. + 1.0);
        camera.target = target;
        self.camera_target_debug = target;
        camera.up = ship.up();
    }

    pub fn update_rotational_velocity(&mut self) {
        // Iterate over all connected gamepads
        // for (_id, gamepad) in self.gilrs.gamepads() {
        //     // println!("{} is {:?}", gamepad.name(), gamepad.power_info());
        // }

        // let mut active_gamepad = None;

        // Examine new events
        while let Some(Event { id, event, time }) = self.gilrs.next_event() {
            match event {
                // gilrs::EventType::ButtonPressed(_, _) => todo!(),
                // gilrs::EventType::ButtonRepeated(_, _) => todo!(),
                // gilrs::EventType::ButtonReleased(_, _) => todo!(),
                // gilrs::EventType::ButtonChanged(_, _, _) => todo!(),
                gilrs::EventType::AxisChanged(axis, value, code) => {
                    match axis {
                        gilrs::Axis::LeftStickX => {
                            self.rotational_velocity.z = -value;
                        }
                        gilrs::Axis::LeftStickY => {
                            self.rotational_velocity.x = value;
                        }
                        gilrs::Axis::RightZ => {
                            self.rotational_velocity.y = value;
                        }
                        // gilrs::Axis::RightStickX => todo!(),
                        // gilrs::Axis::RightStickY => todo!(),
                        // gilrs::Axis::RightZ => todo!(),
                        // gilrs::Axis::DPadX => todo!(),
                        // gilrs::Axis::DPadY => todo!(),
                        // gilrs::Axis::Unknown => todo!(),
                        _ => {}
                    }
                }
                // gilrs::EventType::Connected => todo!(),
                // gilrs::EventType::Disconnected => todo!(),
                // gilrs::EventType::Dropped => todo!(),
                _ => {}
            };
            // println!("{:?} New event from {}: {:?}", time, id, event);
            // active_gamepad = Some(id);
        }
    }

    pub fn mouse_motion(
        &mut self,
        delta: (f64, f64),
        // size: &PhysicalSize<u32>,
        // camera: &mut Camera,
    ) {
        self.rotated_by_mouse = true;
        self.rotational_velocity = vec3(delta.1 as f32 / 10.0, delta.0 as f32 / 10.0, 0.0);
    }
}

pub fn gravity(position: Vec3) -> Vec3 {
    G * position.normalize_or_zero() / position.length_squared().max(1.0) // Prevent extreamly high gravty when things are close
}

pub fn alt_gravity(distance: f32, a: f32, b: f32) -> f32 {
    ((distance - 1.1 * (a + b)).atan() / (distance - a - b).max(0.0).powi(2)).max(-2.0)
}
