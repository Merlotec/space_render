use std::ops::Range;

use amethyst::{
    core::ecs::{
        DispatcherBuilder, World, ReadStorage, Entity,
    },
    core::math::{Matrix4, Vector3, Vector4},
    error::Error,
    renderer::{
        camera::Camera,
        bundle::{RenderOrder, RenderPlan, RenderPlugin, Target},
        pipeline::{PipelineDescBuilder, PipelinesBuilder},
        rendy::{
            command::{QueueId, RenderPassEncoder},
            factory::{Factory, ImageState},
            graph::{
                GraphContext,
                NodeBuffer, NodeImage, render::{PrepareResult, RenderGroup, RenderGroupDesc},
            },
            hal::{self, device::Device,  pso, pso::ShaderStageFlags, query},
            mesh::{AsVertex, Position, TexCoord, PosTex},
            shader::{Shader, SpirvShader},
            texture::{Texture, TextureBuilder, pixel::Rgba8Srgb},
        },
        submodules::{FlatEnvironmentSub, gather::CameraGatherer},
        types::Backend, util,
    },
};

use std::io::Cursor;
use super::*;
use crate::{
    planet::{
        sub::*,
        PlanetList,
        PlanetData,
    },
    star::sub::*,
    util::*,
};
use amethyst::prelude::WorldExt;

const STATIC_DEPTH: f32 = 0.0;
const STATIC_CROP: f32 = 0.2;

const STATIC_VERTEX_DATA: [PosTex; 4] = [
    PosTex { position: Position([-1.0, -1.0, STATIC_DEPTH]), tex_coord: TexCoord([0.0 + STATIC_CROP, 0.0 + STATIC_CROP]) },
    PosTex { position: Position([-1.0, 1.0, STATIC_DEPTH]), tex_coord: TexCoord([0.0 + STATIC_CROP, 1.0 - STATIC_CROP]) },
    PosTex { position: Position([1.0, 1.0, STATIC_DEPTH]), tex_coord: TexCoord([1.0 - STATIC_CROP, 1.0 - STATIC_CROP]) },
    PosTex { position: Position([1.0, -1.0, STATIC_DEPTH]), tex_coord: TexCoord([1.0 - STATIC_CROP, 0.0 + STATIC_CROP]) },
];

const STATIC_INSTANCE_DATA: [u32; 6] = [0, 1, 2, 0, 3, 2];


lazy_static::lazy_static! {
    // These uses the precompiled shaders.
    // These can be obtained using glslc.exe in the vulkan sdk.
    static ref VERTEX: SpirvShader = SpirvShader::new(
        include_bytes!("../../shaders/spirv/star.vert.spv").to_vec(),
        ShaderStageFlags::VERTEX,
        "main",
    );

    static ref FRAGMENT: SpirvShader = SpirvShader::new(
        include_bytes!("../../shaders/spirv/star.frag.spv").to_vec(),
        ShaderStageFlags::FRAGMENT,
        "main",
    );
}

/// Draw triangles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawStarDesc;

impl DrawStarDesc {
    /// Create instance of `DrawStarDesc` render group
    pub fn new() -> Self {
        Default::default()
    }
}

impl<B: Backend> RenderGroupDesc<B, World> for DrawStarDesc {
    fn build(
        self,
        ctx: &GraphContext<B>,
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
        let stars = StarSub::new(
            factory,
            hal::pso::ShaderStageFlags::VERTEX | hal::pso::ShaderStageFlags::FRAGMENT
        )?;
        let tex = load_glow_texture(factory, queue, ctx.frames_in_flight as usize, hal::pso::ShaderStageFlags::FRAGMENT)?;
        
        // Load billboard mesh.
        let vertex = StaticVertexBuffer::allocate(factory, queue, &STATIC_VERTEX_DATA, Some(&STATIC_INSTANCE_DATA))?;
        let (pipeline, pipeline_layout) = build_custom_pipeline(
            factory,
            subpass,
            framebuffer_width,
            framebuffer_height,
            vec![env.raw_layout(), stars.raw_layout(), tex.raw_layout()],
            None,
        )?;

        // let query_pool = factory.device().create_query_pool(
        //     query::Type::Occlusion,
        //     1,
        // );

        Ok(Box::new(DrawStar::<B> {
            pipeline,
            pipeline_layout,
            env,
            vertex,
            stars,
            tex,
            // query_pool,
        }))
    }
}

/// Draws triangles to the screen.
#[derive(Debug)]
pub struct DrawStar<B: Backend> {
    pipeline: B::GraphicsPipeline,
    pipeline_layout: B::PipelineLayout,
    env: FlatEnvironmentSub<B>,
    vertex: StaticVertexBuffer<B, PosTex>,
    stars: StarSub<B>,
    tex: TextureSet<B>,
    // query_pool: B::QueryPool,
}

impl<B: Backend> RenderGroup<B, World> for DrawStar<B> {
    fn prepare(
        &mut self,
        factory: &Factory<B>,
        _queue: QueueId,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        world: &World,
    ) -> PrepareResult {

        self.env.process(factory, index, world);
        self.stars.process(factory, index, world);

        // let mut query_data = Vec::with_capacity(4);
        // factory.device().get_query_pool_results(
        //     &self,
        //     pool: &self.query_pool,
        //     queries: ..,
        //     data: &mut query_data,
        //     stride: 4,
        //     flags: query::ResultFlags::none(),
        // );

        // let count: u32 = unsafe {
        //     *(query_data.as_ptr() as *const u32)
        // };
        

        PrepareResult::DrawRecord
    }

    fn draw_inline(
        &mut self,
        mut encoder: RenderPassEncoder<'_, B>,
        index: usize,
        _subpass: hal::pass::Subpass<'_, B>,
        _world: &World,
    ) {
        // let query = query::Query<B> {
        //     pool: &self.query_pool,
        //     id: qid,
        // };
        // hal::command::RawCommandBuffer::begin_query(
        //     query, 
        //     flags: query::ControlFlags::none(),
        // );
        if !self.stars.is_empty() {
            encoder.bind_graphics_pipeline(&self.pipeline);
            self.env.bind(index, &self.pipeline_layout, 0, &mut encoder);
            self.stars.bind(index, &self.pipeline_layout, 1, &mut encoder);
            self.tex.bind(index, &self.pipeline_layout, 2, &mut encoder);
            unsafe {
                self.vertex.draw(&mut encoder, 0..self.stars.count() as u32);
            }
        }
        // hal::command::RawCommandBuffer::end_query(query);
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
                    fun: pso::Comparison::Less,
                    write: true,
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
#[derive(Debug, Default)]
pub struct StarRender;

impl<B: Backend> RenderPlugin<B> for StarRender {
    fn on_build<'a, 'b>(
        &mut self,
        world: &mut World,
        _builder: &mut DispatcherBuilder<'a, 'b>,
    ) -> Result<(), Error> {
        // Add the required components to the world ECS
        // We need to move the object out of the option to obtain it validly.
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
            ctx.add(RenderOrder::LinearPostEffects, DrawStarDesc::new().builder())?;
            Ok(())
        });
        Ok(())
    }
}

fn load_glow_texture<B: Backend>(factory: &mut Factory<B>, queue: QueueId, image_count: usize, flags: hal::pso::ShaderStageFlags) -> Result<TextureSet<B>, failure::Error> {
    let img_data = include_bytes!("../../assets/star_glow.png");

    let img = image::load(Cursor::new(&img_data[..]), image::PNG)
        .unwrap()
        .to_rgba();
    let (width, height) = img.dimensions();
    let image_data: Vec<Rgba8Srgb> = img
            .pixels()
            .map(|p| Rgba8Srgb { repr: p.0 })
            .collect::<_>();

    let mut tex = TextureSet::new(factory, flags)?;
    let builder = TextureBuilder::new()
        .with_data(image_data.as_slice())
        .with_data_width(width)
        .with_data_height(height)
        .with_view_kind(hal::image::ViewKind::D2)
        .with_kind(hal::image::Kind::D2(width, height, 1, 1));
    // Write the texture per image.
    for i in 0..image_count {
        let glow_tex = builder
        .build(
            ImageState {
                queue,
                stage: hal::pso::PipelineStage::FRAGMENT_SHADER,
                access: hal::image::Access::SHADER_READ,
                layout: hal::image::Layout::ShaderReadOnlyOptimal,
            },
            factory
        )?;
        tex.write_unique(factory, i, glow_tex);
    }
    Ok(tex)
 }