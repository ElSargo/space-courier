use bevy_ecs::{
    schedule::Schedule,
    system::{Resource, SystemState},
    world::World,
};

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

        // SystemState::update_archetypes(, )

        ecs
    }

    pub fn update(&mut self) {
        self.update.run(&mut self.world);
        self.extract.run(&mut self.world);
    }
}

trait Test {
    fn get_state<A, B, C>(f: fn(A, B, C), world: &mut World) -> SystemState<(A, B, C)>
    where
        A: bevy_ecs::system::SystemParam,
        B: bevy_ecs::system::SystemParam,
        C: bevy_ecs::system::SystemParam,
    {
        SystemState::new(world)
    }
}
