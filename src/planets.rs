use std::{borrow::BorrowMut, default::Default};

use glam::{Quat, Vec3};

#[derive(Default, Debug)]
pub struct Planet {
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
    pub radius: f32,
    pub mass: f32,
}

#[derive(Default, Debug)]
pub struct Planets {
    pub translations: Vec<Vec3>,
    pub rotations: Vec<Quat>,
    pub velocities: Vec<Vec3>,
    pub radii: Vec<f32>,
    pub masses: Vec<f32>,
}

impl FromIterator<Planet> for Planets {
    fn from_iter<T: IntoIterator<Item = Planet>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let mut res = match iter.size_hint() {
            (_, Some(len)) => Self::with_capacity(len),
            (len, None) => Self::with_capacity(len),
            _ => Self::default(),
        };
        for planet in iter {
            res.push(planet);
        }
        res
    }
}

impl Planets {
    pub fn with_capacity(size: usize) -> Self {
        Self {
            translations: Vec::with_capacity(size),
            rotations: Vec::with_capacity(size),
            velocities: Vec::with_capacity(size),
            radii: Vec::with_capacity(size),
            masses: Vec::with_capacity(size),
        }
    }

    pub fn len(&self) -> usize {
        self.translations.len()
    }

    pub fn push(&mut self, planet: Planet) {
        self.translations.push(planet.translation);
        self.rotations.push(planet.rotation);
        self.velocities.push(planet.velocity);
        self.radii.push(planet.radius);
        self.masses.push(planet.mass);
    }

    // pub fn get(&self, index: usize) -> Option<PlanetView> {
    //     (index < self.translations.len()).then_some(PlanetView {
    //         translation: &self.translations[index],
    //         rotation: &self.rotations[index],
    //         velocity: &self.velocities[index],
    //         scale: &self.scales[index],
    //         masse: &self.masses[index],
    //     })
    // }
    // pub fn get_mut(&mut self, index: usize) -> Option<PlanetViewMut> {
    //     (index < self.translations.len()).then_some(PlanetViewMut {
    //         translation: &mut self.translations[index],
    //         rotation: &mut self.rotations[index],
    //         velocity: &mut self.velocities[index],
    //         scale: &mut self.scales[index],
    //         masse: &mut self.masses[index],
    //     })
    // }

    // pub fn iter_mut(&mut self) -> PlanetIterator<'_> {
    //     PlanetIterator {
    //         iter: &mut &self,
    //         i: 0,
    //     }
    // }

    // pub fn iter_transforms_mut(&mut self) -> PlanetTransformIterator<'_> {
    //     PlanetTransformIterator {
    //         iter: &mut &self,
    //         i: 0,
    //     }
    // }
}
