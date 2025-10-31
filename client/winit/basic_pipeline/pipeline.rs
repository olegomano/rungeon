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

extern crate vulkan_context;

mod rust_fragment_shader;
mod rust_vertex_shader;

struct BasicPipeline {
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

impl BasicPipeline {
    pub fn new(context: &vulkan_context::VulkanContext, window: &winit::window::Window) -> Self {
        unsafe {
            let (swapchain, surface_format, extent) = Self::create_swapchain(window, context);

            let swapchain_images = context
                .logical_device
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
                    context
                        .logical_device
                        .create_image_view(&create_view_info, None)
                        .expect("Failed to create image view")
                })
                .collect::<Vec<_>>();

            let render_pass = Self::create_render_pass(context, surface_format);

            let pipeline = Self::create_render_pipeline(&window, context, render_pass);

            // More initialization code would go here...

            Self {
                swapchain,
                swapchain_images,
                swapchain_image_views,
                framebuffers: vec![],
                pipeline,
                render_pass,
                command_pool: vk::CommandPool::null(),
                command_buffers: vec![],
                image_available_semaphore: vk::Semaphore::null(),
                render_finished_semaphore: vk::Semaphore::null(),
            }
        }
    }

    unsafe fn create_swapchain(
        window: &winit::window::Window,
        vulkan_context: &vulkan_context::VulkanContext,
    ) -> (vk::SwapchainKHR, vk::Format, vk::Extent2D) {
        let surface_capabilities = vulkan_context
            .instance
            .get_physical_device_surface_capabilities_khr(
                vulkan_context.physical_device,
                vulkan_context.surface,
            )
            .expect("Failed to get surface capabilities");
        let surface_formats = vulkan_context
            .instance
            .get_physical_device_surface_formats_khr(
                vulkan_context.physical_device,
                vulkan_context.surface,
            )
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

        let queue_indecies = [vulkan_context.graphics_queue_index as u32];
        let info = vk::SwapchainCreateInfoKHR::builder()
            .surface(vulkan_context.surface)
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
            vulkan_context
                .logical_device
                .create_swapchain_khr(&info, None)
                .expect("Failed to create swapchain"),
            surface_format.format,
            extent,
        );
    }

    unsafe fn create_render_pass(
        vulkan_context: &vulkan_context::VulkanContext,
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

        return vulkan_context
            .logical_device
            .create_render_pass(&info, None)
            .expect("Failed to create render pass");
    }

    unsafe fn create_render_pipeline(
        window: &winit::window::Window,
        vulkan_context: &vulkan_context::VulkanContext,
        render_pass: vk::RenderPass,
    ) -> vk::Pipeline {
        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vulkan_context.create_shader_module(rust_vertex_shader::VERT_SHADER))
            .name(b"main\0");

        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(vulkan_context.create_shader_module(rust_fragment_shader::FRAG_SHADER))
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
        let layout = vulkan_context
            .logical_device
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

        return vulkan_context
            .logical_device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
            .expect("Failed to create graphics pipeline")
            .0[0];
    }
}
