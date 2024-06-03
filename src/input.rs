use std::collections::HashMap;

use bevy_ecs::{
    event::{Event, Events},
    system::Resource,
};
use gilrs::Gilrs;
use winit::event::VirtualKeyCode;

use crate::ecs::Ecs;

#[derive(Debug, Event)]
pub struct MouseDelta {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy)]
enum KeyState {
    Up,
    JustDown,
    Down,
    JustUp,
}

#[derive(Resource)]
pub struct ButtonInput {
    data: HashMap<VirtualKeyCode, KeyState>,
}

#[allow(dead_code)]
impl ButtonInput {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn on_key_down(&mut self, key: VirtualKeyCode) {
        let key_state = self.data.entry(key).or_insert(KeyState::JustDown);
        match key_state {
            KeyState::Down | KeyState::JustDown => {}
            KeyState::Up | KeyState::JustUp => {
                *key_state = KeyState::JustDown;
            }
        };
    }

    pub fn on_key_up(&mut self, key: VirtualKeyCode) {
        let key_state = self.data.entry(key).or_insert(KeyState::JustUp);
        match key_state {
            KeyState::Up | KeyState::JustUp => {}
            KeyState::Down | KeyState::JustDown => {
                *key_state = KeyState::JustUp;
            }
        };
    }

    pub fn is_down(&self, key: VirtualKeyCode) -> bool {
        match self.data.get(&key) {
            Some(KeyState::JustDown | KeyState::Down) => true,
            _ => false,
        }
    }

    pub fn just_down(&self, key: VirtualKeyCode) -> bool {
        match self.data.get(&key) {
            Some(KeyState::JustDown) => true,
            _ => false,
        }
    }

    pub fn just_up(&self, key: VirtualKeyCode) -> bool {
        match self.data.get(&key) {
            Some(KeyState::JustUp) => true,
            _ => false,
        }
    }

    /// Call at the end of update
    pub fn tick(&mut self) {
        for (_, state) in self.data.iter_mut() {
            *state = match state {
                KeyState::JustDown | KeyState::Down => KeyState::Down,
                KeyState::JustUp | KeyState::Up => KeyState::Up,
            }
        }
    }
}

#[derive(Debug, Clone, Resource, Default)]
pub struct AxisInput {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl AxisInput {
    pub fn update(&mut self, gilrs: &mut Gilrs) {
        while let Some(gilrs::Event { event, .. }) = gilrs.next_event() {
            match event {
                gilrs::EventType::AxisChanged(axis, value, _code) => match axis {
                    gilrs::Axis::LeftStickX => {
                        self.x = value;
                    }
                    gilrs::Axis::LeftStickY => {
                        self.y = value;
                    }
                    gilrs::Axis::RightZ => {
                        self.z = value;
                    }
                    _ => {}
                },
                _ => {}
            };
        }
    }
}

pub fn init(ecs: &mut Ecs) {
    ecs.world.insert_resource(ButtonInput::new());
    ecs.world.insert_resource(AxisInput::default());
    ecs.world.init_resource::<Events<MouseDelta>>();
}
