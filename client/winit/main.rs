use clap::Parser;

extern crate basic_pipeline;
extern crate handle;
extern crate indirect_pipeline;
extern crate primitives;
extern crate vulkan_context;
extern crate vulkan_texture;

use nalgebra::{Matrix4, Vector3};

use window::WinitApp;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Which pipeline to use: "basic" or "indirect"
    #[arg(short, long, default_value = "basic")]
    pipeline: String,
}

enum PipelineVariant {
    Basic(basic_pipeline::BasicPipeline),
    Indirect(indirect_pipeline::IndirectPipeline),
}

struct MainRenderer {
    context: Option<vulkan_context::VulkanContext>,
    pipeline: Option<PipelineVariant>,
    pipeline_kind: String,

    sample_texture: Option<vulkan_texture::VulkanTexture>,

    // WASD rotation state
    angle_x: f32,
    angle_y: f32,
}

impl window::WinitRenderer for MainRenderer {
    fn Init(&mut self, window: &winit::window::Window) {
        unsafe {
            let ctx = vulkan_context::VulkanContext::new(window);

            let mut pipeline_variant = match self.pipeline_kind.as_str() {
                "indirect" => PipelineVariant::Indirect(indirect_pipeline::IndirectPipeline::new(
                    &ctx, window,
                )),
                _ => PipelineVariant::Basic(basic_pipeline::BasicPipeline::new(&ctx, window)),
            };

            let img_bytes = include_bytes!("common/knight.png");
            let img = image::load_from_memory(img_bytes)
                .expect("Failed to load knight.png")
                .into_rgba8();
            let texture = vulkan_texture::VulkanTexture::from_rgba8(
                &ctx,
                img.width(),
                img.height(),
                &img.into_raw(),
            );

            if let PipelineVariant::Indirect(ref mut indirect) = pipeline_variant {
                indirect.update_texture(&ctx, 0, &texture);
                indirect.update_texture(&ctx, 1, &texture);
            }

            self.context = Some(ctx);
            self.pipeline = Some(pipeline_variant);
            self.sample_texture = Some(texture);
        }
    }

    fn Render(&mut self) {
        let ctx = self.context.as_ref().unwrap();

        let view_matrix = Matrix4::from_axis_angle(&Vector3::x_axis(), self.angle_x.to_radians())
            * Matrix4::from_axis_angle(&Vector3::y_axis(), self.angle_y.to_radians());

        match self.pipeline.as_mut().unwrap() {
            PipelineVariant::Basic(basic) => {
                // Move the camera back slightly so we can see the cube at origin
                let final_view =
                    Matrix4::new_translation(&Vector3::new(0.0, 0.0, 2.0)) * view_matrix;
                basic.SetCameraView(ctx, final_view);
                basic.Render(ctx);
            }
            PipelineVariant::Indirect(indirect) => {
                // The indirect pipeline uses an orthographic projection centered at top-left.
                // We'll move our "cube" (it renders a shared quad, but we can pass a matrix)
                // to the center of the screen and scale it up.
                let w = ctx.swapchain_extent.width as f32;
                let h = ctx.swapchain_extent.height as f32;

                let transform = Matrix4::new_translation(&Vector3::new(w / 2.0, h / 2.0, 0.0))
                    * Matrix4::new_scaling(200.0)
                    * view_matrix;

                let sprite = indirect_pipeline::SpriteInstance {
                    // It uses a placeholder white pixel texture internally if we pass dummy
                    texture: handle::handle_t::default(),
                    transform,
                    atlas_rect: [0.0, 0.0, 1.0, 1.0],
                };

                indirect.Draw(ctx, &[sprite]);
            }
        }
    }

    fn Tick(&mut self) {}

    fn OnKeyboardInput(&mut self, input: &KeyEvent) {
        if input.state == ElementState::Pressed {
            match input.physical_key {
                PhysicalKey::Code(KeyCode::KeyW) => self.angle_x -= 5.0,
                PhysicalKey::Code(KeyCode::KeyS) => self.angle_x += 5.0,
                PhysicalKey::Code(KeyCode::KeyA) => self.angle_y -= 5.0,
                PhysicalKey::Code(KeyCode::KeyD) => self.angle_y += 5.0,
                _ => {}
            }
        }
    }
}

fn main() {
    let args = Args::parse();

    let renderer = MainRenderer {
        context: None,
        pipeline: None,
        pipeline_kind: args.pipeline,
        sample_texture: None,
        angle_x: 0.0,
        angle_y: 0.0,
    };

    WinitApp::new(renderer).Run();
}
