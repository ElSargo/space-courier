use glam::Vec3;
use winit::event::VirtualKeyCode;

use crate::camera::Camera;

pub struct CameraController {
    pub speed: f32,
    pub is_forward_pressed: bool,
    pub is_backward_pressed: bool,
    pub is_left_pressed: bool,
    pub is_right_pressed: bool,
    pub is_down_pressed: bool,
    pub is_up_pressed: bool,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_down_pressed: false,
            is_up_pressed: false,
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

    pub fn update_camera(&self, camera: &mut Camera, delta: f32) {
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

        camera.transform.translation += vel * delta;
    }
}
