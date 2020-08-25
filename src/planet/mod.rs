pub mod pass;
pub mod sub;

use amethyst::{
    assets::PrefabData,
    derive::PrefabData,
    ecs::prelude::*,
    renderer::palette::Srgb,
    error::Error,
    core::math::{
        Vector3,
    }
};

use glsl_layout::*;

use serde::{Serialize, Deserialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
/// Describes a planet's planet (from a rendering perspective).
pub struct Atmosphere {
    /// The radius of the atmosphere relative to the planet's radius.
    pub height: f32,

    /// The radius of the base planet.
    pub base_planet_radius: f32,

    /// Hue color of the planet.
    #[serde(with = "amethyst::renderer::serde_shim::srgb")]
    pub hue: Srgb,

    /// The degree of obfuscation which is invoked by this planet.
    pub density: f32,
}

impl Atmosphere {
    /// Create a new planet component with the specified data.
    pub fn new(height: f32, hue: Srgb, density: f32, base_planet_radius: f32) -> Self {
        Self { height, hue, density, base_planet_radius }
    }

    #[inline]
    /// Gets the height of this planet.
    pub fn height(&self) -> f32 {
        self.height
    }

    #[inline]
    /// Gets the hue for this planet.
    pub fn hue(&self) -> Srgb {
        self.hue
    }

    #[inline]
    /// Gets the density of this planet.
    pub fn density(&self) -> f32 {
        self.density
    }
}

impl Component for Atmosphere {
    // We want flagged storage because we need to know when to rebuild the buffers.
    type Storage = DenseVecStorage<Self>;
}


#[derive(Debug, Copy, Clone, Serialize, Deserialize, PrefabData)]
#[prefab(Component)]
/// Describes a planet's planet (from a rendering perspective).
pub struct Planet {
    /// Hue color of the planet.
    pub radius: f32,

    /// The density of the planet.
    pub density: f32,
}

impl Planet {
    /// Create a new planet component with the specified data.
    pub fn new(radius: f32, density: f32) -> Self {
        Self { radius, density }
    }

    #[inline]
    /// Gets the hue for this planet.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    #[inline]
    /// Gets the density of this planet.
    pub fn density(&self) -> f32 {
        self.density
    }
}

impl Component for Planet {
    type Storage = DenseVecStorage<Self>;
}

pub const MAX_PLANETS: usize = 8;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub(crate) struct PlanetData {
    pub center: vec3,
    pub radius: float,
    pub hue: vec3,
    pub atmosphere_radius: float,
    pub atmosphere_density: float,
}

impl PlanetData {
    pub(crate) fn new(atmosphere: &Atmosphere, center: Vector3<f32>, radius: f32) -> Self {
        Self {
            center: Into::<[f32; 3]>::into(center).into(),
            radius,
            hue: [atmosphere.hue.red, atmosphere.hue.green, atmosphere.hue.blue].into(),
            atmosphere_radius: radius * atmosphere.height(),
            atmosphere_density: atmosphere.density(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub(crate) struct PlanetList {
    pub(crate) count: uint,
    planets: [PlanetData; MAX_PLANETS],
}

impl PlanetList {
    pub(crate) fn new(planet_data: &[PlanetData]) -> Self {
        assert!(planet_data.len() <= MAX_PLANETS);
        let mut planets: [PlanetData; MAX_PLANETS] = Default::default();
        for (i, data) in planet_data.iter().enumerate() {
            planets[i] = *data;
        }
        Self { planets, count: planet_data.len() as u32 }
    }
}