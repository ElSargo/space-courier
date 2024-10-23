use std::ops::Deref;

use bevy_ecs::{
    event::{Event, EventReader, Events},
    system::{Res, ResMut, Resource},
};

use std::collections::HashMap;
use winit::{event::AxisId, keyboard::KeyCode};

use crate::{ecs::Initailiser, time::Time};

#[derive(Debug, Clone, Copy, Default)]
struct KeyState {
    down: bool,
    tick: u64,
}

#[derive(Resource, Default)]
pub struct ButtonInput {
    data: HashMap<KeyCode, KeyState>,
    inputs: HashMap<Inputs, KeyState>,
    current_tick: u64,
}

#[allow(dead_code)]
impl ButtonInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(
        mut this: ResMut<Self>,
        time: Res<Time>,
        mut keys: EventReader<KeyEvent>,
        input_map: Res<InputMappings>,
    ) {
        this.current_tick = time.tick;
        for e in keys.read() {
            let entry = this.data.entry(e.code).or_insert(KeyState::default());

            if e.pressed != entry.down {
                entry.tick = time.tick;
            }
            entry.down = e.pressed;

            for input in [None, Some(true), Some(false)]
                .iter()
                .flat_map(|shift| input_map.get(&(e.code, *shift)))
            {
                let entry = this.inputs.entry(*input).or_insert(KeyState {
                    down: false,
                    tick: 0,
                });
                if entry.down != e.pressed {
                    entry.tick = time.tick;
                }
                entry.down = e.pressed;
            }
        }
    }

    pub fn is_down(&self, key: KeyCode) -> bool {
        match self.data.get(&key) {
            Some(KeyState { down, .. }) => *down,
            _ => false,
        }
    }

    pub fn just_down(&self, key: KeyCode) -> bool {
        match self.data.get(&key) {
            Some(KeyState { down, tick }) => *down && *tick == self.current_tick,
            _ => false,
        }
    }

    pub fn just_up(&self, key: KeyCode) -> bool {
        match self.data.get(&key) {
            Some(KeyState { down, tick }) => !down && *tick == self.current_tick,

            _ => false,
        }
    }

    pub fn active(&self, input: Inputs) -> bool {
        match self.inputs.get(&input) {
            Some(KeyState { down, .. }) => *down,
            None => false,
        }
    }

    pub fn just_active(&self, input: Inputs) -> bool {
        match self.inputs.get(&input) {
            Some(KeyState { down, tick }) => *down && *tick == self.current_tick,
            None => false,
        }
    }
}

#[derive(Resource)]
pub struct InputMappings {
    mappings: HashMap<(KeyCode, Option<bool>), Inputs>,
}

impl Deref for InputMappings {
    type Target = HashMap<(KeyCode, Option<bool>), Inputs>;

    fn deref(&self) -> &Self::Target {
        &self.mappings
    }
}

impl Default for InputMappings {
    fn default() -> Self {
        let mut this = HashMap::default();

        this.insert((KeyCode::KeyW, None), Inputs::Forward);
        this.insert((KeyCode::KeyS, None), Inputs::Back);

        Self { mappings: this }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum Inputs {
    Forward,
    Back,
}

#[derive(Debug, Clone, Resource, Default)]
pub struct AxisInput {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Event)]
pub struct AxisEvent {
    pub(crate) axis: AxisId,
    pub(crate) value: f32,
}

pub fn upadte_axis_input(mut axis_input: ResMut<AxisInput>, mut events: EventReader<AxisEvent>) {
    axis_input.x = 0.0;
    axis_input.y = 0.0;
    axis_input.z = 0.0;
    for event in events.read() {
        *(match event.axis {
            0 => &mut axis_input.x,
            1 => &mut axis_input.y,
            2 => &mut axis_input.z,
            _ => {
                continue;
            }
        }) += event.value;
    }
}

pub fn init_mod(ecs: &mut Initailiser) {
    ecs.update.add_systems(ButtonInput::update);
    ecs.world.insert_resource(ButtonInput::new());
    ecs.world.insert_resource(AxisInput::default());
    ecs.world.init_resource::<Events<KeyEvent>>();
    ecs.world.init_resource::<Events<AxisEvent>>();
    ecs.world.init_resource::<InputMappings>();
    ecs.update.add_systems(upadte_axis_input);
}

#[derive(Debug, Event)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub pressed: bool,
}
