[s1]: https://img.shields.io/crates/v/space_render.svg

# space_render
A compact render plugin to the Amethyst game engine which renders different space elements.
It currently supports planet rendering (well, the atmosphere), star/sun rendering and background cosmos rendering (lots of tiny stars).


# How to use
Add `space-render={version="0.1.1", features=["vulkan"]}` to your `cargo.toml` under `[dependencies]`. If you're using a different rendering backend (not vulkan), specify this in the the features.

Then add the required plugins to your Amethyst render bundle as shown:
```rust
use space_render::{
    CosmosRender,
    cosmos::Cosmos,
    AtmosphereRender,
    StarRender,
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
            // We need to include the `CosmosRender` plugin in our rendering bundle in order to render the background stars.
            .with_plugin(CosmosRender::new(Some(Cosmos::default()))),
            // This is the atmosphere renderer.
            .with_plugin(AtmosphereRender::new()),
            // This renders the 'sun' (basicall just a billboard).
            // It does the job far away but it doesn't really work if you get up close.
            // May fix if needed in the future.
            .with_plugin(StarRender::new("asset/path/to/star/image.png")),
    )?;
```
