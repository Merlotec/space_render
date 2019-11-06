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
            mesh::{AsVertex, Position, PosTex, TexCoord},
            shader::{Shader, SpirvShader},
        },
        submodules::FlatEnvironmentSub,
        types::Backend, util,
    },
};
use glsl_layout::*;

use super::*;
use super::util::*;

pub const STAR_DEPTH: f32 = -1000.0;

const STATIC_VERTEX_DATA: [PosTex; 4] = [
    PosTex {
        position: Position([-1.0, -1.0, STAR_DEPTH]),
        tex_coord: TexCoord([0.0, 0.0]),
    },
    PosTex {
        position: Position([-1.0, 1.0, STAR_DEPTH]),
        tex_coord: TexCoord([0.0, 1.0]),
    },
    PosTex {
        position: Position([1.0, 1.0, STAR_DEPTH]),
        tex_coord: TexCoord([1.0, 1.0]),
    },
    PosTex {
        position: Position([1.0, -1.0, STAR_DEPTH]),
        tex_coord: TexCoord([1.0, 0.0]),
    },
];

const STATIC_INSTANCE_DATA: [u32; 6] = [0, 1, 2, 0, 3, 2];

lazy_static::lazy_static! {
    // These uses the precompiled shaders.
    // These can be obtained using glslc.exe in the vulkan sdk.
    static ref VERTEX: SpirvShader = SpirvShader::new(
        include_bytes!("../shaders/spirv/star_point.vert.spv").to_vec(),
        ShaderStageFlags::VERTEX,
        "main",
    );

    static ref FRAGMENT: SpirvShader = SpirvShader::new(
        include_bytes!("../shaders/spirv/star_point.frag.spv").to_vec(),
        ShaderStageFlags::FRAGMENT,
        "main",
    );
}

/// Draw triangles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawCosmosDesc;

impl DrawCosmosDesc {
    /// Create instance of `DrawCosmosDesc` render group
    pub fn new() -> Self {
        Default::default()
    }
}

impl<B: Backend> RenderGroupDesc<B, World> for DrawCosmosDesc {
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
        let star_buffer = DynamicShaderBuffer::new(factory, pso::ShaderStageFlags::VERTEX)?;
        let vertex = StaticVertexBuffer::allocate(factory, queue, &STATIC_VERTEX_DATA, Some(&STATIC_INSTANCE_DATA))?;

        let (pipeline, pipeline_layout) = build_custom_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![env.raw_layout(), star_buffer.raw_layout()],
            None,
        )?;

        Ok(Box::new(DrawSky::<B> {
            pipeline,
            pipeline_layout,
            env,
            vertex,
            star_list: Vec::new(),
            star_buffer,
        }))
    }
}

/// Draws triangles to the screen.
#[derive(Debug)]
pub struct DrawSky<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    env: FlatEnvironmentSub<B>,
    vertex: StaticVertexBuffer<B, PosTex>,
    star_list: Vec<StarData>,
    star_buffer: DynamicShaderBuffer<B, StarData>,
}

impl<B: Backend> RenderGroup<B, World> for DrawSky<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) -> PrepareResult {

        if let Some(mut sky) = world.try_fetch_mut::<Cosmos>() {
            if sky.changed || self.star_list.len() != sky.stars().len() {
                let stars: &[StarPoint] = sky.stars();
                let mut star_vec = Vec::with_capacity(stars.len());
                for star in stars {
                    star_vec.push(StarData::from(*star));
                }
                self.star_list = star_vec;
                self.star_buffer.invalidate();
            }
            sky.changed = false;
            self.env.process(factory, index, world);
            if !self.star_buffer.contains_image_at(index) {
                let changed = self.star_buffer.write_formatted(factory, index, self.star_list.as_slice());
                if changed {
                    PrepareResult::DrawRecord
                } else {
                    PrepareResult::DrawReuse
                }
            } else {
                PrepareResult::DrawReuse
            }
        } else {
            self.star_list.clear();
            PrepareResult::DrawRecord
        }
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        _world: &World,
    ) {
        if self.star_list.len() != 0 {
            encoder.bind_graphics_pipeline(&self.pipeline);
            self.env.bind(index, &self.pipeline_layout, 0, &mut encoder);
            self.star_buffer.bind(index, &self.pipeline_layout, 1, &mut encoder);
            unsafe {
                self.vertex.draw(&mut encoder, 0..self.star_list.len() as u32);
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
                .with_vertex_desc(&[(PosTex::vertex(), pso::VertexInputRate::Vertex)])
                .with_input_assembler(pso::InputAssemblerDesc::new(hal::Primitive::TriangleList))
                // Add the shaders
                .with_shaders(util::simple_shader_set(
                    &shader_vertex,
                    Some(&shader_fragment),
                ))
                .with_layout(&pipeline_layout)
                .with_subpass(subpass)
                .with_framebuffer_size(framebuffer_width, framebuffer_height)
                .with_depth_test(pso::DepthTest::On {
                    fun: pso::Comparison::LessEqual,
                    write: false,
                })
                .with_blend_targets(vec![pso::ColorBlendDesc(pso::ColorMask::ALL, pso::BlendState::ALPHA)]),
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
#[derive(Debug)]
pub struct CosmosRender {
    sky: Option<Cosmos>,
}

impl CosmosRender {
    pub fn new(sky: Option<Cosmos>) -> Self {
        Self { sky }
    }
}

impl<B: Backend> RenderPlugin<B> for CosmosRender {
    fn on_build<'a, 'b>(
        &mut self,
        world: &mut World,
        _builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        // Add the required components to the world ECS
        // We need to move the object out of the option to obtain it validly.
        if let Some(sky) = self.sky.take() {
            world.insert(sky);
        }
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
            ctx.add(RenderOrder::AfterOpaque, DrawCosmosDesc::new().builder())?;
            Ok(())
        });
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd, AsStd140)]
#[repr(C, align(4))]
pub struct StarData {
    pub spherical_coords: vec2,
    pub color: vec3,
    pub scale: float,
}

impl From<StarPoint> for StarData {
    fn from(point: StarPoint) -> Self {
        Self {
            spherical_coords: Into::<[f32; 2]>::into(point.spherical_coords).into(),
            color: [point.color.red, point.color.green, point.color.blue].into(),
            scale: point.radius,
        }
    }
}