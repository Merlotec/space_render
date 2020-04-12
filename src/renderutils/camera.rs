use amethyst::{
    ecs::prelude::*,
    renderer::{
        camera::{Camera, Projection},
        submodules::gather::CameraGatherer,
    }
};

pub fn set_camera_far(world: &World, far: f32) {
     // Change the camera projection to include depth.
        if let Some(camera_entity) = CameraGatherer::gather_camera_entity(world) {
            let mut cameras = world.write_storage::<Camera>();
            if let Some(camera) = cameras.get_mut(camera_entity) {
                let matrix = camera.as_matrix();
                let fov = 2.0 * ( 1.0 as f32 / matrix.row(1)[1]).atan();
                let aspect = matrix.row(1)[1] / matrix.row(0)[0];
                *camera = Camera::from(Projection::perspective(
                    aspect,
                    //std::f32::consts::FRAC_PI_3,
                    fov,
                    0.1,
                    far,
                ))
            }
        }
 }