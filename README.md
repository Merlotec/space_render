# space_render
A compact render plugin to the Amethyst game engine which renders different space elements.
It currently supports planet rendering (well, the atmosphere), star/sun rendering and background star rendering (lots of tiny stars).


# How to use
Add the required plugins to your Amethyst render bundle as shown:
```rust
use space_render::{
    cosmos::{Cosmos, CosmosRender},
    planet::PlanetRender,
    star::StarRender,
};

let display_config_path = app_root.join("config\\display.ron");

let game_data = GameDataBuilder::default()
    // Add all your other bundles here:
    // ...
    // Setup the rendering bundle.
    .with_bundle(
        RenderingBundle::<DefaultBackend>::new()
            // Here you add whatever other rendering plugins you want to use.
            // The following are necessary for 3D pbr rendering:
            //.with_plugin(RenderToWindow::from_config_path(display_config_path).with_clear([0.0, 0.0, 0.0, 0.0]))
            //.with_plugin(RenderPbr3D::default().with_skinning())
            // We need to include the `CosmosRender` plugin in our rendering bundle.
            .with_plugin(CosmosRender::new(Some(Cosmos::default()))),
            .with_plugin(PlanetRender::new()),
            .with_plugin(StarRender::new()),
    )?;
```
