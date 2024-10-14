use crate::prelude::*;
use crate::standard_object;

#[derive(Component)]
pub struct Ship;

pub fn init_mod(ecs: &mut Initailiser) {
    standard_object!(ecs, "retroShip.obj", Ship, With<Ship>);
}
