pub mod planet;
pub mod star;
pub mod cosmos;

mod renderutils;

pub use renderutils::set_camera_far;

pub use planet::{
    Planet,
    Atmosphere,
};

pub use star::Star;

pub use planet::pass::AtmosphereRender;
pub use cosmos::pass::CosmosRender;
pub use star::pass::StarRender;