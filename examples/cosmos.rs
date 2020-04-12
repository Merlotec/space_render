use amethyst::{winit::{
    VirtualKeyCode,
},window::ScreenDimensions, renderer::{
    RenderingBundle,
    RenderPbr3D,
    RenderToWindow,
    camera::Camera,
    types::DefaultBackend,
}, input::{
    InputBundle,
    StringBindings,
    is_close_requested,
    is_key_down,
}, core::{
    TransformBundle,
    Transform,
}, utils::{
    auto_fov::AutoFovSystem,
    application_root_dir,
}, controls::{
    FlyControlBundle,
    FlyControlTag,
}, GameDataBuilder, SimpleState, StateData, GameData, Application, Error, SimpleTrans, Trans, StateEvent};
use space_render::cosmos::{
    Cosmos,
    CosmosRender,
};
use amethyst::prelude::{World, WorldExt, Builder};

fn main() -> Result<(), Error> {
    // Get the application root directory for asset loading.
    let app_root = application_root_dir()?;

    // Add our meshes directory to the asset loader.
    let assets_dir = app_root.join("assets");

    // Load display config
    let display_config_path = app_root.join("config\\display.ron");

    let game_data = GameDataBuilder::new()
        .with_bundle(InputBundle::<StringBindings>::new())?
        .with(
            AutoFovSystem::new(),
            "auto_fov",
            &[],
        )
        .with_bundle(
            FlyControlBundle::<StringBindings>::new(None, None, None)
                .with_sensitivity(0.1, 0.1)
                .with_speed(5.0),
        )?
        .with_bundle(TransformBundle::new())?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(RenderToWindow::from_config_path(display_config_path)?.with_clear([0.0; 4]))
                .with_plugin(RenderPbr3D::default())
                // We crate a new `Cosmos` which will be placed in a resource.
                // Alternatively you can use `Cosmos::with_random_distribution(1000)` to provide an arbitrary number of stars.
                .with_plugin(CosmosRender::new(Some(Cosmos::default())))

        )?;
    let mut app = Application::new(assets_dir, CosmosState, game_data)?;
    app.run();
    Ok(())
}

pub struct CosmosState;

impl SimpleState for CosmosState {
    fn on_start(&mut self, data: StateData<'_, GameData>) {
        let world: &mut World = data.world;

        let (width, height) = {
            let dims = world.read_resource::<ScreenDimensions>();
            (dims.width(), dims.height())
        };

        world.create_entity()
            .with(Camera::standard_3d(width, height))
            .with(Transform::default())
            .with(FlyControlTag)
            .build();
    }

    fn handle_event(&mut self, _data: StateData<'_, GameData>, event: StateEvent) -> SimpleTrans {
        if let StateEvent::Window(window_event) = event {
            if is_close_requested(&window_event) || is_key_down(&window_event, VirtualKeyCode::Escape) {
                Trans::Quit
            } else {
                Trans::None
            }
        } else {
            Trans::None
        }
    }
}