use anyhow::anyhow;
use anyhow::Result;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::window as vk_window;
use window::WinitApp;
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;

// This struct holds the Vulkan resources that need initialization
#[derive(Debug)]
struct VulkanContext {
    entry: Entry,
    instance: Instance,
}

impl VulkanContext {
    unsafe fn new(window: &winit::window::Window) -> Self {
        let loader = LibloadingLoader::new(LIBRARY).expect("Failed to load Vulkan library");
        let entry = Entry::new(loader).expect("Failed to create entry");
        let instance = Self::create_instance(window, &entry);
        return Self { entry, instance };
    }

    unsafe fn create_instance(window: &winit::window::Window, entry: &Entry) -> Instance {
        let application_info = vk::ApplicationInfo::builder()
            .application_name(b"Vulkan Tutorial\0")
            .application_version(vk::make_version(1, 0, 0))
            .engine_name(b"No Engine\0")
            .engine_version(vk::make_version(1, 0, 0))
            .api_version(vk::make_version(1, 0, 0));

        let extensions = vk_window::get_required_instance_extensions(window)
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<_>>();

        let info = vk::InstanceCreateInfo::builder()
            .application_info(&application_info)
            .enabled_extension_names(&extensions);

        return entry
            .create_instance(&info, None)
            .expect("Failed to create instance");
    }
}

// The main renderer that can be default constructed
#[derive(Debug, Default)]
struct ExampleRenderer {
    context: Option<VulkanContext>,
}

impl ExampleRenderer {
    fn InitVulkan(&mut self, window: &winit::window::Window) {
        unsafe {
            self.context = Some(VulkanContext::new(window));
        }
    }
}

impl window::WinitRenderer for ExampleRenderer {
    fn Init(&mut self, window: &winit::window::Window) {
        self.InitVulkan(window);
    }

    fn Render(&mut self) {}
    fn Tick(&mut self) {}
    fn OnKeyboardInput(&mut self, input: &KeyEvent) {}
}

fn main() {
    let renderer = ExampleRenderer::default();
    WinitApp::new(renderer).Run();
}
