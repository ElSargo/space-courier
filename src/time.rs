use std::time::{Duration, Instant};

use bevy_ecs::system::{ResMut, Resource};

use crate::ecs::Ecs;

#[allow(dead_code)]
#[derive(Resource)]
pub struct Time {
    startup: Instant,
    last_frame: Instant,
    now: Instant,
    pub delta: Duration,
    pub delta_seconds: f32,
}

impl Time {
    pub(crate) fn init() -> Self {
        let now = std::time::Instant::now();
        Self {
            startup: now,
            last_frame: now,
            now,
            delta: Duration::new(0, 0),
            delta_seconds: 0.0,
        }
    }

    fn update(&mut self) {
        self.last_frame = self.now;
        self.now = std::time::Instant::now();
        self.delta = self.now.duration_since(self.last_frame);
        self.delta_seconds = self.delta.as_secs_f32();
    }
}

fn tick(mut time: ResMut<Time>) {
    time.update();
}

pub async fn init(ecs: &mut Ecs) {
    ecs.world.insert_resource(Time::init());
    ecs.update.add_systems(tick);
}
