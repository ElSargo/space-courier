use bevy_ecs::{schedule::Schedule, world::World};

pub struct Ecs {
    pub world: World,
    pub update: Schedule,
    pub init: Schedule,
    pub extract: Schedule,
}

impl Ecs {
    pub async fn new() -> Self {
        let world = World::default();
        let init = Schedule::default();
        let update = Schedule::default();

        let extract = Schedule::default();

        let ecs = Self {
            world,
            update,
            extract,
            init,
        };

        ecs
    }

    pub fn update(&mut self) {
        self.update.run(&mut self.world);
    }
}
