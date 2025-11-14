use anyhow::anyhow;
use anyhow::Result;
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

extern crate rust_shader;

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
    framebuffers: Vec<vk::Framebuffer>,

    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,

    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,

    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
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

        let (swapchain, surface_format, swapchain_extent) =
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

        let render_pass = Self::create_render_pass(&logical_device, &instance, surface_format);
        let pipeline = Self::create_render_pipeline(window, &logical_device, render_pass);

        let framebuffers = swapchain_image_views
            .iter()
            .map(|i| {
                let attachments = &[*i];
                let create_info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(attachments)
                    .width(swapchain_extent.width)
                    .height(swapchain_extent.height)
                    .layers(1);

                return logical_device.create_framebuffer(&create_info, None);
            })
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to create framebuffers");

        let command_pool_info = vk::CommandPoolCreateInfo::builder().queue_family_index(
            Self::find_queue_index(&instance, physical_device, vk::QueueFlags::GRAPHICS)
                .expect("Failed to find graphics queue index") as u32,
        );
        let command_pool = logical_device
            .create_command_pool(&command_pool_info, None)
            .expect("Failed to create command pool");

        let command_buffers = Self::create_command_buffers(
            &logical_device,
            pipeline,
            render_pass,
            &framebuffers,
            command_pool,
            swapchain_extent,
        );

        let semaphore_info = vk::SemaphoreCreateInfo::builder();
        let image_available_semaphore = logical_device
            .create_semaphore(&semaphore_info, None)
            .expect("Failed to create semaphore");
        let render_finished_semaphore = logical_device
            .create_semaphore(&semaphore_info, None)
            .expect("Failed to create semaphore");
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
            framebuffers: framebuffers,

            pipeline: pipeline,
            render_pass: render_pass,

            command_pool: command_pool,
            command_buffers: command_buffers,

            image_available_semaphore: image_available_semaphore,
            render_finished_semaphore: render_finished_semaphore,
        };
    }

    unsafe fn create_swapchain(
        window: &winit::window::Window,
        instance: &Instance,
        logical_device: &Device,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
    ) -> (vk::SwapchainKHR, vk::Format, vk::Extent2D) {
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

        return (
            logical_device
                .create_swapchain_khr(&info, None)
                .expect("Failed to create swapchain"),
            surface_format.format,
            extent,
        );
    }

    unsafe fn create_render_pipeline(
        window: &winit::window::Window,
        logical_device: &Device,
        render_pass: vk::RenderPass,
    ) -> vk::Pipeline {
        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(Self::create_shader_module(
                logical_device,
                rust_vertex_shader::VERT_SHADER,
            ))
            .name(b"main\0");

        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(Self::create_shader_module(
                logical_device,
                rust_fragment_shader::FRAG_SHADER,
            ))
            .name(b"main\0");

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder();

        // Input Assembly State

        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let extent = vk::Extent2D::builder()
            .width(window.inner_size().width)
            .height(window.inner_size().height)
            .build();

        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(0.0)
            .width(extent.width as f32)
            .height(extent.height as f32)
            .min_depth(0.0)
            .max_depth(1.0);

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(extent);

        let viewports = &[viewport];
        let scissors = &[scissor];
        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewports(viewports)
            .scissors(scissors);

        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false);

        // Multisample State

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::_1);

        // Color Blend State

        let attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(false);

        let attachments = &[attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let layout_info = vk::PipelineLayoutCreateInfo::builder();
        let layout = logical_device
            .create_pipeline_layout(&layout_info, None)
            .expect("Failed to create pipeline layout");

        let stages = &[vert_stage, frag_stage];
        let info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(layout)
            .render_pass(render_pass)
            .subpass(0);

        return logical_device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
            .expect("Failed to create graphics pipeline")
            .0[0];
    }

    unsafe fn create_render_pass(
        device: &Device,
        instance: &Instance,
        surface_format: vk::Format,
    ) -> vk::RenderPass {
        let color_attachment = vk::AttachmentDescription::builder()
            .format(surface_format)
            .samples(vk::SampleCountFlags::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        // Subpasses

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_attachments = &[color_attachment_ref];
        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(color_attachments);

        // Create

        let attachments = &[color_attachment];
        let subpasses = &[subpass];
        let info = vk::RenderPassCreateInfo::builder()
            .attachments(attachments)
            .subpasses(subpasses);

        return device
            .create_render_pass(&info, None)
            .expect("Failed to create render pass");
    }

    unsafe fn create_command_buffers(
        device: &Device,
        pipeline: vk::Pipeline,
        render_pass: vk::RenderPass,
        framebuffers: &Vec<vk::Framebuffer>,
        command_pool: vk::CommandPool,
        extent: vk::Extent2D,
    ) -> Vec<vk::CommandBuffer> {
        // Allocate

        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(framebuffers.len() as u32);

        let command_buffers = device
            .allocate_command_buffers(&allocate_info)
            .expect("Failed to allocate command buffers");

        // Commands

        for (i, command_buffer) in command_buffers.iter().enumerate() {
            let info = vk::CommandBufferBeginInfo::builder();

            device
                .begin_command_buffer(*command_buffer, &info)
                .expect("Failed to begin command buffer");

            let render_area = vk::Rect2D::builder()
                .offset(vk::Offset2D::default())
                .extent(extent);

            let color_clear_value = vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            };

            let clear_values = &[color_clear_value];
            let info = vk::RenderPassBeginInfo::builder()
                .render_pass(render_pass)
                .framebuffer(framebuffers[i])
                .render_area(render_area)
                .clear_values(clear_values);

            device.cmd_begin_render_pass(*command_buffer, &info, vk::SubpassContents::INLINE);
            device.cmd_bind_pipeline(*command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline);
            device.cmd_draw(*command_buffer, 3, 1, 0, 0);
            device.cmd_end_render_pass(*command_buffer);

            device
                .end_command_buffer(*command_buffer)
                .expect("Failed to end command buffer");
        }

        return command_buffers;
    }

    unsafe fn create_shader_module(device: &Device, bytecode: &[u8]) -> vk::ShaderModule {
        let code = Bytecode::new(bytecode).expect("Failed to create bytecode from shader");
        let info = vk::ShaderModuleCreateInfo::builder()
            .code(bytecode.align_to::<u32>().1)
            .code_size(bytecode.len());
        return device
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

    fn Render(&mut self) {
        if let Some(context) = &self.context {
            unsafe {
                let device = &self.context.as_ref().unwrap().logical_device;
                let swapchain = self.context.as_ref().unwrap().swapchain;
                let command_buffers = &self.context.as_ref().unwrap().command_buffers;
                let image_available_semaphore =
                    self.context.as_ref().unwrap().image_available_semaphore;
                let render_finished_semaphore =
                    self.context.as_ref().unwrap().render_finished_semaphore;

                let (image_index, _) = device
                    .acquire_next_image_khr(
                        swapchain,
                        std::u64::MAX,
                        image_available_semaphore,
                        vk::Fence::null(),
                    )
                    .expect("Failed to acquire next image");

                let wait_semaphores = &[image_available_semaphore];
                let signal_semaphores = &[render_finished_semaphore];
                let command_buffers_to_submit = &[command_buffers[image_index as usize]];

                let submit_info = vk::SubmitInfo::builder()
                    .wait_semaphores(wait_semaphores)
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .command_buffers(command_buffers_to_submit)
                    .signal_semaphores(signal_semaphores);

                device
                    .queue_submit(
                        self.context.as_ref().unwrap().graphics_queue,
                        &[submit_info],
                        vk::Fence::null(),
                    )
                    .expect("Failed to submit to queue");

                let swapchains = &[swapchain];
                let image_indices = &[image_index];
                let present_info = vk::PresentInfoKHR::builder()
                    .wait_semaphores(signal_semaphores)
                    .swapchains(swapchains)
                    .image_indices(image_indices);

                device
                    .queue_present_khr(self.context.as_ref().unwrap().present_queue, &present_info)
                    .expect("Failed to present queue");

                device
                    .queue_wait_idle(self.context.as_ref().unwrap().present_queue)
                    .expect("Failed to wait queue idle");
            }
        }
    }
    fn Tick(&mut self) {}
    fn OnKeyboardInput(&mut self, input: &KeyEvent) {}
}

fn main() {
    let renderer = ExampleRenderer::default();
    WinitApp::new(renderer).Run();
}
