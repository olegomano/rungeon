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

    pub descriptor_pool: vk::DescriptorPool,

    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,

    pub surface_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,

    // Common extracted physical device properties
    pub device_name: String,
    pub api_version_major: u32,
    pub api_version_minor: u32,
    pub api_version_patch: u32,
    pub vendor_id: u32,
    pub device_id: u32,
    pub device_type: vk::PhysicalDeviceType,

    // Useful limits / compute info
    pub max_image_dimension_2d: u32,
    pub max_compute_work_group_invocations: u32,
    pub max_compute_work_group_size: [u32; 3],
    pub max_compute_shared_memory_size: u32,

    // Queue family compute information
    pub compute_queue_family_count: u32,
    pub compute_queue_count: u32,
}

impl VulkanContext {
    const DEVICE_EXTENSIONS: &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];
    const VALIDATION_ENABLED: bool = true;
    const VALIDATION_LAYER: vk::ExtensionName =
        vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");

    pub unsafe fn new(window: &winit::window::Window) -> Self {
        let loader = LibloadingLoader::new(LIBRARY).expect("Failed to load Vulkan library");
        let entry = Entry::new(loader).expect("Failed to create entry");
        let instance = Self::create_instance(window, &entry);
        let physical_device = Self::pick_physical_device(&instance);
        let logical_device = Self::create_logical_device(&entry, &instance, physical_device);
        let graphics_queue_index =
            Self::find_queue_index(&instance, physical_device, vk::QueueFlags::GRAPHICS);

        let graphics_queue = logical_device.get_device_queue(graphics_queue_index as u32, 0);
        let descriptor_pool = Self::create_descriptor_pool(&logical_device);
        let surface = vk_window::create_surface(&instance, &window, &window)
            .expect("Failed to create surface");

        let (swapchain, surface_format, swapchain_extent) = Self::create_swapchain(
            window,
            &physical_device,
            &surface,
            &logical_device,
            &instance,
            graphics_queue_index,
        );

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

        // Extract some common properties and limits from the chosen physical
        // device so callers can use them without extra Vulkan calls.
        let properties = instance.get_physical_device_properties(physical_device);
        let device_name = unsafe { CStr::from_ptr(properties.device_name.as_ptr()) }
            .to_string_lossy()
            .into_owned();

        let api_version = properties.api_version;
        let api_version_major = (api_version >> 22) & 0x3ff;
        let api_version_minor = (api_version >> 12) & 0x3ff;
        let api_version_patch = api_version & 0xfff;

        let vendor_id = properties.vendor_id;
        let device_id = properties.device_id;
        let device_type = properties.device_type;

        let limits = properties.limits;
        let max_image_dimension_2d = limits.max_image_dimension_2d;
        let max_compute_work_group_invocations = limits.max_compute_work_group_invocations;
        let max_compute_work_group_size = [
            limits.max_compute_work_group_size[0],
            limits.max_compute_work_group_size[1],
            limits.max_compute_work_group_size[2],
        ];
        let max_compute_shared_memory_size = limits.max_compute_shared_memory_size;

        // Count compute-capable queue families and sum of queue counts
        let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
        let mut compute_queue_family_count = 0u32;
        let mut compute_queue_count = 0u32;
        for q in &queue_families {
            if q.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                compute_queue_family_count += 1;
                compute_queue_count += q.queue_count as u32;
            }
        }

        return Self {
            entry,
            instance,
            physical_device,
            logical_device,
            surface: surface,
            graphics_queue_index,
            graphics_queue,
            present_queue: graphics_queue,
            descriptor_pool: descriptor_pool,
            swapchain: swapchain,
            swapchain_images: swapchain_images,
            surface_format: surface_format,
            swapchain_extent: swapchain_extent,
            swapchain_image_views: swapchain_image_views,

            device_name,
            api_version_major,
            api_version_minor,
            api_version_patch,
            vendor_id,
            device_id,
            device_type,

            max_image_dimension_2d,
            max_compute_work_group_invocations,
            max_compute_work_group_size,
            max_compute_shared_memory_size,

            compute_queue_family_count,
            compute_queue_count,
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

    /// Returns the VkPhysicalDeviceProperties for the selected physical device.
    ///
    /// This is a small safe wrapper around the raw Vulkan call so callers can
    /// easily inspect properties such as device name, limits, and API version.
    pub fn physical_device_properties(&self) -> vk::PhysicalDeviceProperties {
        unsafe {
            self.instance
                .get_physical_device_properties(self.physical_device)
        }
    }

    unsafe fn create_descriptor_pool(logical_device: &Device) -> vk::DescriptorPool {
        // --- Define the total capacity of individual descriptors ---
        let pool_sizes = vec![
            // 1. Uniform Buffers (e.g., for 100 per-object matrices)
            vk::DescriptorPoolSize::builder()
                .type_(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(100),
            // 2. Combined Image Samplers (e.g., for 50 textures)
            vk::DescriptorPoolSize::builder()
                .type_(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(50),
            // You would add other types like STORAGE_BUFFER, etc., here if needed.
        ];
        let max_sets = 200;
        let info = vk::DescriptorPoolCreateInfo::builder()
            .flags(vk::DescriptorPoolCreateFlags::empty()) // Can add flags like FREE_DESCRIPTOR_SET
            .max_sets(max_sets)
            .pool_sizes(&pool_sizes)
            .build();

        return logical_device
            .create_descriptor_pool(&info, None)
            .expect("Failed to create descriptor pool");
    }

    unsafe fn create_swapchain(
        window: &winit::window::Window,
        physical_device: &vk::PhysicalDevice,
        surface: &vk::SurfaceKHR,
        logical_device: &Device,
        instance: &Instance,
        graphics_queue_index: i32,
    ) -> (vk::SwapchainKHR, vk::Format, vk::Extent2D) {
        let surface_capabilities = instance
            .get_physical_device_surface_capabilities_khr(*physical_device, *surface)
            .expect("Failed to get surface capabilities");
        let surface_formats = instance
            .get_physical_device_surface_formats_khr(*physical_device, *surface)
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

        let queue_indecies = [graphics_queue_index as u32];
        let info = vk::SwapchainCreateInfoKHR::builder()
            .surface(*surface)
            .min_image_count(surface_capabilities.min_image_count + 1)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_indecies)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(vk::PresentModeKHR::FIFO)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());

        return (
            logical_device
                .create_swapchain_khr(&info, None)
                .expect("Failed to create swapchain"),
            surface_format.format,
            extent,
        );
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
