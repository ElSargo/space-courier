use gilrs::{Event, Gilrs};
use glam::{Quat, Vec3};
use winit::event::VirtualKeyCode;

use crate::camera::Camera;

#[derive(Debug)]
pub struct CameraController {
    pub speed: f32,
    pub velocity: Vec3,
    pub rotational_velocity: Vec3,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
    pub is_down_pressed: bool,
    pub is_up_pressed: bool,
    pub gilrs: Gilrs,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            gilrs: Gilrs::new().unwrap(),
            velocity: Default::default(),
            rotational_velocity: Default::default(),
            is_forward_pressed: Default::default(),
            is_backward_pressed: Default::default(),
            is_left_pressed: Default::default(),
            is_right_pressed: Default::default(),
            is_down_pressed: Default::default(),
            is_up_pressed: Default::default(),
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

    pub fn update_camera(&mut self, camera: &mut Camera, delta: f32) {
        let mut vel = Vec3::ZERO;
        if self.is_forward_pressed {
            vel += camera.transform.forward()
        }
        if self.is_backward_pressed {
            vel += camera.transform.back()
        }
        if self.is_left_pressed {
            vel += camera.transform.left()
        }
        if self.is_right_pressed {
            vel += camera.transform.right()
        }
        if self.is_up_pressed {
            vel += camera.transform.up()
        }
        if self.is_down_pressed {
            vel += camera.transform.down()
        }
        self.velocity += vel * delta;
        self.velocity += delta * gravity(camera.transform.translation);
        camera.transform.translation += self.velocity * delta * self.speed;

        camera.transform.rotation = camera.transform.rotation
            * Quat::from_euler(
                glam::EulerRot::XYZ,
                self.rotational_velocity.x * 0.01,
                self.rotational_velocity.y * 0.01,
                self.rotational_velocity.z * 0.01,
            );
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
}

fn gravity(position: Vec3) -> Vec3 {
    -0.001 * position.normalize_or_zero() * position.length().max(1.0).powi(2)
}
