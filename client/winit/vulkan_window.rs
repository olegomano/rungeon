use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::ExtDebugUtilsExtension;
use vulkanalia::vk::KhrSurfaceExtension;
use vulkanalia::vk::KhrSwapchainExtension;
use vulkanalia::window as vk_window;
use window::WinitApp;
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::platform::wayland::WindowExtWayland;

extern crate pipeline;
extern crate vulkan_context;

// The main renderer that can be default constructed
#[derive(Debug, Default)]
struct ExampleRenderer {
    context: Option<vulkan_context::VulkanContext>,
    pipeline: Option<pipeline::BasicPipeline>,
}

impl window::WinitRenderer for ExampleRenderer {
    fn Init(&mut self, window: &winit::window::Window) {
        unsafe {
            self.context = Some(vulkan_context::VulkanContext::new(window));
            self.pipeline = Some(pipeline::BasicPipeline::new(
                &self.context.as_ref().unwrap(),
                window,
            ));
        }
    }

    fn Render(&mut self) {
        self.pipeline
            .as_mut()
            .unwrap()
            .Render(self.context.as_ref().unwrap());
    }

    fn Tick(&mut self) {}
    fn OnKeyboardInput(&mut self, input: &KeyEvent) {}
}

fn main() {
    let renderer = ExampleRenderer::default();
    WinitApp::new(renderer).Run();
}
