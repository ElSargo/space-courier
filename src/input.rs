use std::collections::HashMap;

use bevy_ecs::{
    event::{Event, EventReader, Events},
    system::{Res, ResMut, Resource},
};
use gilrs::Gilrs;
use winit::keyboard::Key;

use crate::{ecs::Initailiser, time::Time};

#[derive(Debug, Event)]
pub struct MouseDelta {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct KeyState {
    down: bool,
    tick: u64,
}

#[derive(Resource)]
pub struct ButtonInput {
    data: HashMap<Key, KeyState>,
    current_tick: u64,
}

#[allow(dead_code)]
impl ButtonInput {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            current_tick: 0,
        }
    }

    // pub fn on_key_down(&mut self, key: Key) {
    //     let key_state = self.data.entry(key).or_insert(KeyState::JustDown);
    //     match key_state {
    //         KeyState::Down | KeyState::JustDown => {}
    //         KeyState::Up | KeyState::JustUp => {
    //             *key_state = KeyState::JustDown;
    //         }
    //     };
    // }

    // pub fn on_key_up(&mut self, key: Key) {
    //     let key_state = self.data.entry(key).or_insert(KeyState::JustUp);
    //     match key_state {
    //         KeyState::Up | KeyState::JustUp => {}
    //         KeyState::Down | KeyState::JustDown => {
    //             *key_state = KeyState::JustUp;
    //         }
    //     };
    // }

    pub fn update(mut this: ResMut<Self>, time: Res<Time>, mut keys: EventReader<KeyEvent>) {
        this.current_tick = time.tick;
        for e in keys.read() {
            let entry = this
                .data
                .entry(e.0.logical_key.clone())
                .or_insert(KeyState::default());

            entry.tick = time.tick;
            entry.down = e.0.state.is_pressed();
        }
    }

    pub fn is_down(&self, key: Key) -> bool {
        match self.data.get(&key) {
            Some(KeyState { down, .. }) => *down,
            _ => false,
        }
    }

    pub fn just_down(&self, key: Key) -> bool {
        match self.data.get(&key) {
            Some(KeyState { down, tick }) => *down && *tick == self.current_tick,
            _ => false,
        }
    }

    pub fn just_up(&self, key: Key) -> bool {
        match self.data.get(&key) {
            Some(KeyState { down, tick }) => !down && *tick == self.current_tick,

            _ => false,
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

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.update.add_systems(ButtonInput::update);
    ecs.world.insert_resource(ButtonInput::new());
    ecs.world.insert_resource(AxisInput::default());
    ecs.world.init_resource::<Events<MouseDelta>>();
    ecs.world.init_resource::<Events<KeyEvent>>();
}

#[derive(Debug, Event)]
pub struct KeyEvent(pub winit::event::KeyEvent);
