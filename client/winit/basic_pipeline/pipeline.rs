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

extern crate nalgebra;
use nalgebra::{Matrix4, Point3, Quaternion, Unit, UnitQuaternion, Vector3, Vector4};

extern crate primitives;
extern crate vulkan_buffer;
extern crate vulkan_context;

mod rust_fragment_shader;
mod rust_vertex_shader;

#[derive(Copy, Clone, Debug)]
struct PositionUvBinding {
    binding: vk::VertexInputBindingDescription,
    position: vk::VertexInputAttributeDescription,
    uv: vk::VertexInputAttributeDescription,
}

#[derive(Copy, Clone, Debug, Default)]
struct CameraUbo {
    view: Matrix4<f32>,
}

/*
* Is a descriptor of mapping a VulkanBuffer to a shader input
*/
impl PositionUvBinding {
    unsafe fn new(binding_index: u32, vertex_location: u32, uv_location: u32) -> Self {
        let binding = vk::VertexInputBindingDescription::builder()
            .binding(binding_index)
            .stride(24 as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build();

        let vertex = vk::VertexInputAttributeDescription::builder()
            .binding(binding_index)
            .location(vertex_location)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let uv = vk::VertexInputAttributeDescription::builder()
            .binding(binding_index)
            .location(uv_location)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(12)
            .build();

        return Self {
            binding,
            position: vertex,
            uv: uv,
        };
    }
}

#[derive(Debug)]
pub struct BasicPipeline {
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

    camera_ubo: vulkan_buffer::VulkanObject<CameraUbo>,
}

impl BasicPipeline {
    pub fn new(context: &vulkan_context::VulkanContext, window: &winit::window::Window) -> Self {
        unsafe {
            let mut camera_ubo = vulkan_buffer::VulkanObject::<CameraUbo>::new(context);
            camera_ubo.data.view = Matrix4::identity();
            camera_ubo.Sync(context);

            let (swapchain, surface_format, swapchain_extent) =
                Self::create_swapchain(window, context);

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

                    return context
                        .logical_device
                        .create_framebuffer(&create_info, None);
                })
                .collect::<Result<Vec<_>, _>>()
                .expect("Failed to create framebuffers");

            let (descriptor_set, descriptor_set_layout) =
                Self::create_descriptor_set(context, &camera_ubo.buffer.buffer);

            let (pipeline, pipeline_layout) =
                Self::create_render_pipeline(&window, context, render_pass, descriptor_set_layout);

            let command_pool_info = vk::CommandPoolCreateInfo::builder()
                .queue_family_index(context.graphics_queue_index as u32);

            let command_pool = context
                .logical_device
                .create_command_pool(&command_pool_info, None)
                .expect("Failed to create command pool");

            let command_buffers = Self::create_command_buffers(
                context,
                pipeline,
                pipeline_layout,
                render_pass,
                &framebuffers,
                command_pool,
                swapchain_extent,
                descriptor_set,
            );

            let semaphore_info = vk::SemaphoreCreateInfo::builder();
            let image_available_semaphore = context
                .logical_device
                .create_semaphore(&semaphore_info, None)
                .expect("Failed to create semaphore");
            let render_finished_semaphore = context
                .logical_device
                .create_semaphore(&semaphore_info, None)
                .expect("Failed to create semaphore");

            Self {
                swapchain,
                swapchain_images,
                swapchain_image_views,
                framebuffers: framebuffers,
                pipeline,
                render_pass,
                command_pool: command_pool,
                command_buffers: command_buffers,
                image_available_semaphore: image_available_semaphore,
                render_finished_semaphore: render_finished_semaphore,
                camera_ubo: camera_ubo,
            }
        }
    }

    pub fn Render(&mut self, context: &vulkan_context::VulkanContext) {
        unsafe {
            let (image_index, _) = context
                .logical_device
                .acquire_next_image_khr(
                    self.swapchain,
                    std::u64::MAX,
                    self.image_available_semaphore,
                    vk::Fence::null(),
                )
                .expect("Failed to acquire next image");

            self.camera_ubo.data.view =
                Matrix4::from_axis_angle(&Vector3::z_axis(), 1.0_f32.to_radians())
                    * self.camera_ubo.data.view;
            self.camera_ubo.Sync(context);

            let wait_semaphores = &[self.image_available_semaphore];
            let signal_semaphores = &[self.render_finished_semaphore];
            let command_buffers_to_submit = &[self.command_buffers[image_index as usize]];

            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(command_buffers_to_submit)
                .signal_semaphores(signal_semaphores);

            context
                .logical_device
                .queue_submit(context.graphics_queue, &[submit_info], vk::Fence::null())
                .expect("Failed to submit to queue");

            let swapchains = &[self.swapchain];
            let image_indices = &[image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(signal_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            context
                .logical_device
                .queue_present_khr(context.present_queue, &present_info)
                .expect("Failed to present queue");

            context
                .logical_device
                .queue_wait_idle(context.present_queue)
                .expect("Failed to wait queue idle");
        }
    }

    unsafe fn create_command_buffers(
        context: &vulkan_context::VulkanContext,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        render_pass: vk::RenderPass,
        framebuffers: &Vec<vk::Framebuffer>,
        command_pool: vk::CommandPool,
        extent: vk::Extent2D,
        ubo_descriptor_set: vk::DescriptorSet,
    ) -> Vec<vk::CommandBuffer> {
        // Allocate

        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(framebuffers.len() as u32);

        let command_buffers = context
            .logical_device
            .allocate_command_buffers(&allocate_info)
            .expect("Failed to allocate command buffers");

        let rect = vulkan_buffer::VulkanBuffer::new(
            context,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            primitives::QUAD_TRIANGLES.len() * std::mem::size_of::<primitives::Vertex>(),
        );
        rect.Write(&primitives::QUAD_TRIANGLES, context);

        for (i, command_buffer) in command_buffers.iter().enumerate() {
            let info = vk::CommandBufferBeginInfo::builder();

            context
                .logical_device
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

            context.logical_device.cmd_begin_render_pass(
                *command_buffer,
                &info,
                vk::SubpassContents::INLINE,
            );
            context.logical_device.cmd_bind_pipeline(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline,
            );
            context.logical_device.cmd_bind_descriptor_sets(
                *command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0, // first_set: This is the 'set=0' index
                &[ubo_descriptor_set],
                &[], // dynamic_offsets: None for a static UBO
            );

            context.logical_device.cmd_bind_vertex_buffers(
                *command_buffer,
                0,
                &[rect.buffer],
                &[0],
            );
            context.logical_device.cmd_draw(
                *command_buffer,
                primitives::QUAD_TRIANGLES.len() as u32,
                1,
                0,
                0,
            );
            context.logical_device.cmd_end_render_pass(*command_buffer);

            context
                .logical_device
                .end_command_buffer(*command_buffer)
                .expect("Failed to end command buffer");
        }

        return command_buffers;
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
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vulkan_context.create_shader_module(rust_vertex_shader::VERT_SHADER))
            .name(b"main\0");

        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(vulkan_context.create_shader_module(rust_fragment_shader::FRAG_SHADER))
            .name(b"main\0");

        let shader_binding = PositionUvBinding::new(0, 0, 1);
        let binding_descriptions = &[shader_binding.binding];
        let attribute_descriptions = &[shader_binding.uv, shader_binding.position];

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(binding_descriptions)
            .vertex_attribute_descriptions(attribute_descriptions);

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

        let descriptor_set_layouts = [descriptor_set_layout];

        let layout_info =
            vk::PipelineLayoutCreateInfo::builder().set_layouts(&descriptor_set_layouts);

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

        return (
            vulkan_context
                .logical_device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[info], None)
                .expect("Failed to create graphics pipeline")
                .0[0],
            layout,
        );
    }

    unsafe fn create_descriptor_set(
        vulkan_context: &vulkan_context::VulkanContext,
        camera_ubo: &vk::Buffer,
    ) -> (vk::DescriptorSet, vk::DescriptorSetLayout) {
        // Define the binding directly in the list
        let ubo_bindings = [vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS)
            .build()];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&ubo_bindings); // Reference the local array

        let layout = vulkan_context
            .logical_device
            .create_descriptor_set_layout(&layout_info, None)
            .expect("Failed to create descriptor set layout");

        // Allocate the set
        let layouts = [layout];
        let allocate_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(vulkan_context.descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_set = vulkan_context
            .logical_device
            .allocate_descriptor_sets(&allocate_info)
            .expect("Failed to allocate descriptor sets")[0];

        // Update/Write the set
        let buffer_info = [vk::DescriptorBufferInfo::builder()
            .buffer(*camera_ubo)
            .offset(0)
            .range(vk::WHOLE_SIZE as u64)
            .build()];

        let write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_info); // Use reference to local array
        let copy: [vk::CopyDescriptorSet; 0] = [];

        vulkan_context
            .logical_device
            .update_descriptor_sets(&[*write], &copy);

        (descriptor_set, layout)
    }
}
