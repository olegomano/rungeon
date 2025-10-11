use anyhow::anyhow;
use anyhow::Result;
use std::collections::HashSet;
use std::ffi::CStr;
use std::os::raw::c_void;
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

extern crate triangle_frag;
extern crate triangle_vert;

// This struct holds the Vulkan resources that need initialization
#[derive(Debug)]
struct VulkanContext {
    entry: Entry,
    instance: Instance,
    physical_device: vk::PhysicalDevice,
    logical_device: Device,
    surface: vk::SurfaceKHR,

    graphics_queue: vk::Queue,
    present_queue: vk::Queue,

    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
}

impl VulkanContext {
    const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];
    const VALIDATION_ENABLED: bool = true;
    const VALIDATION_LAYER: vk::ExtensionName =
        vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

    unsafe fn new(window: &winit::window::Window) -> Self {
        let loader = LibloadingLoader::new(LIBRARY).expect("Failed to load Vulkan library");
        let entry = Entry::new(loader).expect("Failed to create entry");
        let instance = Self::create_instance(window, &entry);
        let physical_device = Self::pick_physical_device(&instance);
        let logical_device = Self::create_logical_device(&entry, &instance, physical_device);
        let graphics_queue = logical_device.get_device_queue(
            Self::find_queue_index(&instance, physical_device, vk::QueueFlags::GRAPHICS).expect("")
                as u32,
            0,
        );
        let surface = vk_window::create_surface(&instance, &window, &window)
            .expect("Failed to create surface");

        let swapchain =
            Self::create_swapchain(window, &instance, &logical_device, physical_device, surface);
        let swapchain_images = logical_device
            .get_swapchain_images_khr(swapchain)
            .expect("Failed to get swapchain images");
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(vk::ImageViewType::_2D)
                    .format(vk::Format::B8G8R8A8_SRGB)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build(),
                    );
                logical_device
                    .create_image_view(&create_view_info, None)
                    .expect("Failed to create image view")
            })
            .collect::<Vec<_>>();

        /*
         * Assume graphics and and present queues are the same for now
         */
        return Self {
            entry: entry,
            instance: instance,
            physical_device: physical_device,
            logical_device: logical_device,
            surface: surface,
            graphics_queue: graphics_queue,
            present_queue: graphics_queue,
            swapchain: swapchain,
            swapchain_images: swapchain_images,
            swapchain_image_views: swapchain_image_views,
        };
    }

    unsafe fn create_swapchain(
        window: &winit::window::Window,
        instance: &Instance,
        logical_device: &Device,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> vk::SwapchainKHR {
        let surface_capabilities = instance
            .get_physical_device_surface_capabilities_khr(physical_device, surface)
            .expect("Failed to get surface capabilities");
        let surface_formats = instance
            .get_physical_device_surface_formats_khr(physical_device, surface)
            .expect("Failed to get surface formats");

        let surface_format = surface_formats
            .iter()
            .cloned()
            .find(|f| {
                f.format == vk::Format::B8G8R8A8_SRGB
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or_else(|| surface_formats[0]);

        let extent = vk::Extent2D::builder()
            .width(window.inner_size().width.clamp(
                surface_capabilities.min_image_extent.width,
                surface_capabilities.max_image_extent.width,
            ))
            .height(window.inner_size().height.clamp(
                surface_capabilities.min_image_extent.height,
                surface_capabilities.max_image_extent.height,
            ))
            .build();

        let queue_family_indices =
            [
                Self::find_queue_index(instance, physical_device, vk::QueueFlags::GRAPHICS)
                    .expect("Failed to find graphics queue index") as u32,
            ];

        let info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(surface_capabilities.min_image_count + 1)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family_indices)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        return logical_device
            .create_swapchain_khr(&info, None)
            .expect("Failed to create swapchain");
    }

    //print all the devices and choose the first one
    unsafe fn pick_physical_device(instance: &Instance) -> vk::PhysicalDevice {
        let devices = instance
            .enumerate_physical_devices()
            .expect("Failed to enumerate physical devices");
        assert!(!devices.is_empty(), "No Vulkan physical devices found");

        for d in &devices {
            let properties = instance.get_physical_device_properties(*d);
            let device_name = CStr::from_ptr(properties.device_name.as_ptr());
            println!("Found device: {:?}", device_name);
        }
        return devices[0];
    }

    unsafe fn create_logical_device(
        entry: &Entry,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> Device {
        let mut extensions = Self::DEVICE_EXTENSIONS
            .iter()
            .map(|n| n.as_ptr())
            .collect::<Vec<_>>();

        let graphics_queue_index =
            Self::find_queue_index(instance, physical_device, vk::QueueFlags::GRAPHICS)
                .expect("Failed to find graphics queue index");

        let queue_priorities = [1.0_f32];
        let queue_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_index as u32)
            .queue_priorities(&queue_priorities);

        let device_features = vk::PhysicalDeviceFeatures::builder();

        let layers = if Self::VALIDATION_ENABLED {
            vec![Self::VALIDATION_LAYER.as_ptr()]
        } else {
            Vec::new()
        };

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(std::slice::from_ref(&queue_info))
            .enabled_layer_names(&layers)
            .enabled_extension_names(&extensions)
            .enabled_features(&device_features);

        let device = instance
            .create_device(physical_device, &device_create_info, None)
            .expect("Failed to create logical device");

        return device;
    }

    unsafe fn find_queue_index(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        queue_flags: vk::QueueFlags,
    ) -> Result<i32> {
        let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
        for (index, queue_family) in queue_families.iter().enumerate() {
            if queue_family.queue_flags.contains(queue_flags) {
                return Ok(index as i32);
            }
        }
        Err(anyhow!("No graphics queue family found"))
    }

    /*
     * Allocates a vulkan API instance
     */
    unsafe fn create_instance(window: &winit::window::Window, entry: &Entry) -> Instance {
        let available_layers = entry
            .enumerate_instance_layer_properties()
            .expect("Failed to enumerate layer properties")
            .iter()
            .map(|l| l.layer_name)
            .collect::<HashSet<_>>();

        let layers = if Self::VALIDATION_ENABLED {
            vec![Self::VALIDATION_LAYER.as_ptr()]
        } else {
            Vec::new()
        };

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
            .enabled_layer_names(&layers)
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
    let frag = triangle_frag::SHADER;
    let vert = triangle_vert::SHADER;

    let renderer = ExampleRenderer::default();
    WinitApp::new(renderer).Run();
}
