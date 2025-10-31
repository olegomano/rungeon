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
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::event_loop::EventLoop;
use winit::platform::wayland::WindowExtWayland;

#[derive(Debug)]
pub struct VulkanContext {
    pub entry: Entry,
    pub instance: Instance,
    pub physical_device: vk::PhysicalDevice,
    pub logical_device: Device,
    pub surface: vk::SurfaceKHR,

    pub graphics_queue_index: i32,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
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
        let graphics_queue_index =
            Self::find_queue_index(&instance, physical_device, vk::QueueFlags::GRAPHICS);

        let graphics_queue = logical_device.get_device_queue(graphics_queue_index as u32, 0);
        let surface = vk_window::create_surface(&instance, &window, &window)
            .expect("Failed to create surface");

        return Self {
            entry,
            instance,
            physical_device,
            logical_device,
            surface: surface,
            graphics_queue_index,
            graphics_queue,
            present_queue: graphics_queue,
        };
    }

    pub unsafe fn create_shader_module(&self, bytecode: &[u8]) -> vk::ShaderModule {
        let code = Bytecode::new(bytecode).expect("Failed to create bytecode from shader");
        let info = vk::ShaderModuleCreateInfo::builder()
            .code(bytecode.align_to::<u32>().1)
            .code_size(bytecode.len());
        return self
            .logical_device
            .create_shader_module(&info, None)
            .expect("Failed to create shader module");
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
            Self::find_queue_index(instance, physical_device, vk::QueueFlags::GRAPHICS);

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
    ) -> i32 {
        let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
        for (index, queue_family) in queue_families.iter().enumerate() {
            if queue_family.queue_flags.contains(queue_flags) {
                return index as i32;
            }
        }
        return 0;
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
