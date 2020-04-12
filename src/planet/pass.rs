use std::ops::Range;

use amethyst::{
    core::ecs::{
        DispatcherBuilder, World,
    },
    error::Error,
    renderer::{
        bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
        pipeline::{PipelineDescBuilder, PipelinesBuilder},
        rendy::{
            command::{QueueId, RenderPassEncoder},
            factory::Factory,
            graph::{
                GraphContext,
                NodeBuffer, NodeImage, render::{PrepareResult, RenderGroup, RenderGroupDesc},
            },
            hal::{self, device::Device,  pso, pso::ShaderStageFlags},
            mesh::{AsVertex, Position},
            shader::{Shader, SpirvShader},
        },
        submodules::{FlatEnvironmentSub},
        types::Backend, util,
    },
};

use crate::{
    planet::sub::*,
    star::sub::*,
};

use crate::renderutils::*;

use amethyst::prelude::WorldExt;

const STATIC_DEPTH: f32 = 0.0;

const STATIC_VERTEX_DATA: [Position; 4] = [
    Position([-1.0, -1.0, STATIC_DEPTH]),
    Position([-1.0, 1.0, STATIC_DEPTH]),
    Position([1.0, 1.0, STATIC_DEPTH]),
    Position([1.0, -1.0, STATIC_DEPTH]),
];

const STATIC_INSTANCE_DATA: [u32; 6] = [0, 1, 2, 0, 3, 2];


lazy_static::lazy_static! {
    // These uses the precompiled shaders.
    // These can be obtained using glslc.exe in the vulkan sdk.
    static ref VERTEX: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../../shaders/spirv/atmosphere.vert.spv"),
        ShaderStageFlags::VERTEX,
        "main",
    ).unwrap();

    static ref FRAGMENT: SpirvShader = SpirvShader::from_bytes(
        include_bytes!("../../shaders/spirv/atmosphere.frag.spv"),
        ShaderStageFlags::FRAGMENT,
        "main",
    ).unwrap();
}

/// Draw triangles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawAtmosphereDesc;

impl DrawAtmosphereDesc {
    /// Create instance of `DrawAtmosphereDesc` render group
    pub fn new() -> Self {
        Default::default()
    }
}

impl<B: Backend> RenderGroupDesc<B, World> for DrawAtmosphereDesc {
    fn build(
        self,
        _ctx: &GraphContext<B>,
        factory: &mut Factory<B>,
        queue: QueueId,
        _world: &World,
        framebuffer_width: u32,
        framebuffer_height: u32,
        subpass: hal::pass::Subpass<'_, B>,
        _buffers: Vec<NodeBuffer>,
        _images: Vec<NodeImage>,
    ) -> Result<Box<dyn RenderGroup<B, World>>, failure::Error> {
        let env = FlatEnvironmentSub::new(factory)?;
        let planets = PlanetSub::new(factory, pso::ShaderStageFlags::FRAGMENT)?;
        let stars = StarSub::new(factory, pso::ShaderStageFlags::FRAGMENT)?;
         // We need to generate the sphere mesh for the planet.
        let vertex = StaticVertexBuffer::allocate(factory, queue, &STATIC_VERTEX_DATA, Some(&STATIC_INSTANCE_DATA))?;
        let (pipeline, pipeline_layout) = build_custom_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![env.raw_layout(), planets.raw_layout(), stars.raw_layout()],
            None,
        )?;

        Ok(Box::new(DrawAtmosphere::<B> {
            pipeline,
            pipeline_layout,
            env,
            vertex,
            planets,
            stars,
        }))
    }
}

/// Draws triangles to the screen.
#[derive(Debug)]
pub struct DrawAtmosphere<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    env: FlatEnvironmentSub<B>,
    vertex: StaticVertexBuffer<B, Position>,
    planets: PlanetSub<B>,
    stars: StarSub<B>,
}

impl<B: Backend> RenderGroup<B, World> for DrawAtmosphere<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) -> PrepareResult {

        self.env.process(factory, index, world);
        self.planets.process(factory, index, world);
        self.stars.process(factory, index, world);

        PrepareResult::DrawRecord
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        _world: &World,
    ) {
        if !self.planets.is_empty() {
            encoder.bind_graphics_pipeline(&self.pipeline);
            self.env.bind(index, &self.pipeline_layout, 0, &mut encoder);
            self.planets.bind(index, &self.pipeline_layout, 1, &mut encoder);
            self.stars.bind(index, &self.pipeline_layout, 2, &mut encoder);
            unsafe {
                self.vertex.draw(&mut encoder, 0..1);
            }
        }
    }

    fn dispose(self: Box<Self>, factory: &mut Factory<B>, _world: &World) {
        unsafe {
            factory.device().destroy_graphics_pipeline(self.pipeline);
            factory
                .device()
                .destroy_pipeline_layout(self.pipeline_layout);
        }
    }
}

fn build_custom_pipeline<B: Backend>(
    factory: &Factory<B>,
    subpass: hal::pass::Subpass<'_, B>,
    framebuffer_width: u32,
    framebuffer_height: u32,
    layouts: Vec<&B::DescriptorSetLayout>,
    push_constant: Option<(hal::pso::ShaderStageFlags, Range<u32>)>,
) -> Result<(B::GraphicsPipeline, B::PipelineLayout), failure::Error> {
    let pipeline_layout = unsafe {
        factory
            .device()
            .create_pipeline_layout(layouts, push_constant)
    }?;
    // Load the shaders
    let shader_vertex = unsafe { VERTEX.module(factory).unwrap() };
    let shader_fragment = unsafe { FRAGMENT.module(factory).unwrap() };

    // Build the pipeline
    let pipes = PipelinesBuilder::new()
        .with_pipeline(
            PipelineDescBuilder::new()
                // This Pipeline uses our custom vertex description and uses instancing.
                .with_vertex_desc(&[(Position::vertex(), pso::VertexInputRate::Vertex)])
                .with_input_assembler(pso::InputAssemblerDesc::new(hal::Primitive::TriangleList))
                // Add the shaders
                .with_shaders(util::simple_shader_set(
                    &shader_vertex,
                    Some(&shader_fragment),
                ))
                .with_layout(&pipeline_layout)
                .with_subpass(subpass)
                .with_framebuffer_size(framebuffer_width, framebuffer_height)
                .with_depth_test(pso::DepthTest {
                    fun: pso::Comparison::Less,
                    write: true,
                })
                .with_blend_targets(vec![pso::ColorBlendDesc { blend: Some(pso::BlendState::ALPHA), mask: pso::ColorMask::ALL}]),
        )
        .build(factory, None);

    // Destoy the shaders once loaded
    unsafe {
        factory.destroy_shader_module(shader_vertex);
        factory.destroy_shader_module(shader_fragment);
    }

    // Handle the Errors
    match pipes {
        Err(e) => {
            unsafe {
                factory.device().destroy_pipeline_layout(pipeline_layout);
            }
            Err(e)
        }
        Ok(mut pipes) => Ok((pipes.remove(0), pipeline_layout)),
    }
}

/// A [RenderPlugin] for our custom plugin
#[derive(Debug, Default)]
pub struct AtmosphereRender;

impl<B: Backend> RenderPlugin<B> for AtmosphereRender {
    fn on_build<'a, 'b>(
        &mut self,
        world: &mut World,
        _builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        // Add the required components to the world ECS
        // We need to move the object out of the option to obtain it validly.
        world.register::<crate::Planet>();
        world.register::<crate::Atmosphere>();
        world.register::<crate::Star>();
        Ok(())
    }

    fn on_plan(
        &mut self,
        plan: &mut RenderPlan<B>,
        _factory: &mut Factory<B>,
        _world: &World,
    ) -> Result<(), Error> {
        plan.extend_target(Target::Main, |ctx| {
            // Add our Description
            ctx.add(RenderOrder::BeforeTransparent, DrawAtmosphereDesc::new().builder())?;
            Ok(())
        });
        Ok(())
    }
}