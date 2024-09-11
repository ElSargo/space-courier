/*
______      _     _____ ________  ___                     _     _   _                            _                                _     _
| ___ \    | |   /  ___|_   _|  \/  |                    | |   | | (_)                          | |                              | |   | |
| |_/ /   _| |_  \ `--.  | | | .  . | __      _____  _ __| | __| |  _ _ __    _ __ ___ _ __   __| | ___ _ __  __      _____  _ __| | __| |
|  __/ | | | __|  `--. \ | | | |\/| | \ \ /\ / / _ \| '__| |/ _` | | | '_ \  | '__/ _ \ '_ \ / _` |/ _ \ '__| \ \ /\ / / _ \| '__| |/ _` |
| |  | |_| | |_  /\__/ /_| |_| |  | |  \ V  V / (_) | |  | | (_| | | | | | | | | |  __/ | | | (_| |  __/ |     \ V  V / (_) | |  | | (_| |
\_|   \__,_|\__| \____/ \___/\_|  |_/   \_/\_/ \___/|_|  |_|\__,_| |_|_| |_| |_|  \___|_| |_|\__,_|\___|_|      \_/\_/ \___/|_|  |_|\__,_|
*/

use bevy_ecs::{
    schedule::{Schedule, ScheduleLabel},
    world::World,
};

pub struct Ecs {
    pub world: World,
    pub update: Schedule,
    pub extract: Schedule,
    pub init: Schedule,
    pub sim_init: Schedule,
}

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct UpdateSchedual;
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct ExtractSchedual;
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct InitSchedual;
#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct SimInitSchedual;

struct SimWorld(World);

impl Ecs {
    pub fn new() -> Self {
        let mut world = World::new();
        world.insert_resource(SimWorld(World::new()));
        let ecs = Self {
            world,
            update: Schedule::new(UpdateSchedual),
            extract: Schedule::new(ExtractSchedual),
            init: Schedule::new(InitSchedual)
                .set_executor_kind(bevy_ecs::schedule::ExecutorKind::Simple),
            sim_init: Schedule::new(SimInitSchedual)
                .set_executor_kind(bevy_ecs::schedule::ExecutorKind::Simple),
        };

        // SystemState::update_archetypes(, )

        ecs
    }

    pub fn update(&mut self) {
        self.update.run(&mut self.world);
        self.extract.run(&mut self.world);
    }
}
