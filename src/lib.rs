pub mod pass;
pub mod util;

pub use pass::CosmosRender;

use amethyst::{
    core::math::Vector2,
    renderer::{
        palette::Srgb,
    },
};
use rand::Rng;

pub const DEFAULT_STAR_COUNT: usize = 4000;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct StarPoint {
    /// Spherical rotation from forward (x, y plane).
    pub spherical_coords: Vector2<f32>,

    /// The color of the star.
    pub color: Srgb,

    /// The radius of the star.
    pub radius: f32,
}

impl StarPoint {
    /// Creates a new star point with the specified spherical rotation, color and radius.
    pub fn new(spherical_coords: Vector2<f32>, color: Srgb, radius: f32) -> Self {
        Self { spherical_coords, color, radius }
    }
}

#[derive(Debug, Clone)]
pub struct Cosmos {
    /// The list of stars (can be auto populated).
    stars: Vec<StarPoint>,

    /// Is set to true when the stars have been changed.
    pub(crate) changed: bool,
}

impl Cosmos {
    /// Creates a new cosmos with the specified custom star points.
    pub fn new(stars: Vec<StarPoint>) -> Self {
        Self { stars, changed: true }
    }

    /// Creates a new cosmos background with a random distribution of stars which exist on a 'sphere' around the world.
    /// It is recommended not to use over about 10000 stars to keep high performance (when rendering in real time).
    pub fn with_random_distribution(count: usize) -> Self {
        // Preallocate star vector.
        let mut stars: Vec<StarPoint> = Vec::with_capacity(count);

        // Create a random number generator with a seed.
        let mut rng = rand::thread_rng();

        // Executes the star creation code for the number of stars.
        // Each star will have different random values.
        for _i in 0..count {

            // We can generate any random x rotation.
            let rx: f32 = rng.gen_range(0.0, std::f32::consts::PI * 2.0);

            // Generating a random y rotation would cause the points to collect at the 'poles'.
            // To take into account this effect, we need to use the sine function to distribute the points.
            // This causes less points to exist near the pole, in accordance with the sine ratio.
            let ry: f32 = (rng.gen_range(-1.0, 1.0) as f32).asin();

            // Place the rotation values into a vector.
            let rot: Vector2<f32> = Vector2::new(rx, ry);

            // Generate a random radius for the star.
            let rad: f32 = rng.gen_range(0.5, 3.0);

            // Generate a random color for the star.
            let r: f32 = rng.gen_range(0.4, 0.8);
            let g: f32 = rng.gen_range(0.6, 0.9);
            let b: f32 = rng.gen_range(0.85, 1.0);

            // Place the individual color values into a color struct.
            let color: Srgb = Srgb::new(r, g, b);

            // Create the star with the random values.
            let star: StarPoint = StarPoint::new(rot, color, rad);

            // Adds the star to the preallocated vector of stars.
            stars.push(star);
        }

        // Creates the 'Cosmos' struct with the generated stars.
        Self::new(stars)
    }

    /// Gets the list of stars in this cosmos.
    pub fn stars(&self) -> &[StarPoint] {
        self.stars.as_slice()
    }

    /// Changes the stars of the `Cosmos`. This requires the data to be reuploaded to the GPU and is not advised.
    pub fn set_stars(&mut self, stars: Vec<StarPoint>) {
        self.stars = stars;
        self.changed = true;
    }
}

impl Default for Cosmos {
    /// Allows a `Cosmos` object to be easily constructed using a default number of stars in random distribution.
    fn default() -> Self {
        Self::with_random_distribution(DEFAULT_STAR_COUNT)
    }
}