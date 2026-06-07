use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::DrawIndexedIndirectCommand;
use vulkanalia::vk::KhrSwapchainExtension;

extern crate nalgebra;
use nalgebra::Matrix4;

extern crate handle;
extern crate primitives;
extern crate vulkan_buffer;
extern crate vulkan_context;
extern crate vulkan_texture;

mod rust_fragment_shader;
mod rust_vertex_shader;

// -----------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------

/// Maximum number of sprites the pipeline can render in a single frame.
const MAX_SPRITES: usize = 4096;

/// Indices for the unit quad (two triangles).
const QUAD_INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

// -----------------------------------------------------------------------
// GPU-layout structs
// -----------------------------------------------------------------------

/// Per-instance data uploaded to the instance attribute buffer.
/// Must match the vertex shader input layout (locations 2-6).
#[repr(C)]
#[derive(Copy, Clone, Debug)]PipelineVariant
pub struct InstanceData {
    /// Model transform (mat4 = 4x vec4 = locations 2,3,4,5)
    pub model: [[f32; 4]; 4],
    /// Atlas sub-rect: (u_offset, v_offset, u_scale, v_scale) (location 6)
    pub atlas_rect: [f32; 4],
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            atlas_rect: [0.0, 0.0, 1.0, 1.0],
        }
    }
}

/// Camera uniform buffer layout. Matches shader set=0 binding=0.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CameraUbo {
    view: [[f32; 4]; 4],
    projection: [[f32; 4]; 4],
}

impl Default for CameraUbo {
    fn default() -> Self {
        let identity: [[f32; 4]; 4] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        Self {
            view: identity,
            projection: identity,
        }
    }
}

// -----------------------------------------------------------------------
// Public input type
// -----------------------------------------------------------------------

/// A sprite that the pipeline should draw.
pub struct SpriteInstance {
    /// Handle into whatever texture store the caller manages.
    pub texture: handle::handle_t<vulkan_texture::VulkanTexture>,
    /// World-space model matrix for this sprite.
    pub transform: Matrix4<f32>,
    /// Sub-rect in the atlas: (u_offset, v_offset, u_scale, v_scale).
    /// Use [0, 0, 1, 1] to sample the entire texture.
    pub atlas_rect: [f32; 4],
}

// -----------------------------------------------------------------------
// Vertex input binding helpers
// -----------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
struct VertexBindingConfig {
    vertex_binding: vk::VertexInputBindingDescription,
    instance_binding: vk::VertexInputBindingDescription,
}

impl VertexBindingConfig {
    /// Build vertex input bindings and attribute descriptions for the pipeline.
    ///
    /// Binding 0: per-vertex data (pos vec4 + uv vec2 = 24 bytes, Vertex rate)
    /// Binding 1: per-instance data (mat4 + vec4 = 80 bytes, Instance rate)
    unsafe fn new() -> Self {
        Self {
            vertex_binding: vk::VertexInputBindingDescription::builder()
                .binding(0)
                .stride(std::mem::size_of::<primitives::Vertex>() as u32)
                .input_rate(vk::VertexInputRate::VERTEX)
                .build(),
            instance_binding: vk::VertexInputBindingDescription::builder()
                .binding(1)
                .stride(std::mem::size_of::<InstanceData>() as u32)
                .input_rate(vk::VertexInputRate::INSTANCE)
                .build(),
        }
    }

    fn bindings(&self) -> [vk::VertexInputBindingDescription; 2] {
        [self.vertex_binding, self.instance_binding]
    }

    /// Returns all 7 attribute descriptions:
    ///  loc 0: position (vec4, binding 0, offset 0)
    ///  loc 1: uv       (vec2, binding 0, offset 16)
    ///  loc 2-5: model mat4 columns (vec4 each, binding 1)
    ///  loc 6: atlas_rect (vec4, binding 1, offset 64)
    fn attributes(&self) -> [vk::VertexInputAttributeDescription; 7] {
        [
            // -- per-vertex --
            // location 0: position (vec4)
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
            // location 1: uv (vec2)
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: 16,
            },
            // -- per-instance --
            // location 2: model col 0
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
            // location 3: model col 1
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 16,
            },
            // location 4: model col 2
            vk::VertexInputAttributeDescription {
                location: 4,
                binding: 1,VertexBindingConfig
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 32,
            },
            // location 5: model col 3
            vk::VertexInputAttributeDescription {
                location: 5,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 48,
            },
            // location 6: atlas_rect (vec4)
            vk::VertexInputAttributeDescription {
                location: 6,
                binding: 1,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 64,
            },
        ]
    }
}

// -----------------------------------------------------------------------
// Double-buffered frame resources
// -----------------------------------------------------------------------

/// Per-frame GPU resources so we can update one while the other is in flight.
/// The command buffers are NOT here — they are recorded once at init and
/// stored per-swapchain-image since they reference specific framebuffers.
#[derive(Debug)]
struct FrameResources {
    instance_buffer: vulkan_buffer::VulkanBuffer,
    indirect_buffer: vulkan_buffer::VulkanBuffer,
    camera_buffer: vulkan_buffer::VulkanBuffer,
    descriptor_set: vk::DescriptorSet,
    fence: vk::Fence,
}

// -----------------------------------------------------------------------
// IndirectPipeline
// -----------------------------------------------------------------------

#[derive(Debug)]
pub struct IndirectPipeline {
    // --- Vulkan pipeline objects ---
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
    descriptor_set_layout: vk::DescriptorSetLayout,

    // --- Shared resources ---
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,

    // Pre-recorded command buffers, one per swapchain image.
    // These never change — the GPU reads draw params from the indirect buffer.
    command_buffers: Vec<vk::CommandBuffer>,

    // Geometry: a single quad shared by all sprites
    vertex_buffer: vulkan_buffer::VulkanBuffer,
    index_buffer: vulkan_buffer::VulkanBuffer,

    // --- Double-buffered frame data ---
    frames: [FrameResources; 2],
    current_frame: usize,

    // --- Synchronisation ---
    image_available_semaphores: [vk::Semaphore; 2],
    render_finished_semaphores: [vk::Semaphore; 2],

    // --- Default texture (white pixel) for sprites that have no texture ---
    default_texture: vulkan_texture::VulkanTexture,
}

impl IndirectPipeline {
    pub fn new(context: &vulkan_context::VulkanContext, window: &winit::window::Window) -> Self {
        unsafe { Self::create(context, window) }
    }

    /// Draw the scene. `sprites` is the full list of sprite instances to
    /// render this frame. Only the GPU buffers are updated — the command
    /// buffers were recorded once at init.
    pub fn Draw(&mut self, context: &vulkan_context::VulkanContext, sprites: &[SpriteInstance]) {
        unsafe {
            self.draw_impl(context, sprites);
        }
    }

    // -------------------------------------------------------------------
    // Creation helpers
    // -------------------------------------------------------------------

    unsafe fn create(
        context: &vulkan_context::VulkanContext,
        window: &winit::window::Window,
    ) -> Self {
        let render_pass = Self::create_render_pass(context);
        let framebuffers = Self::create_framebuffers(context, render_pass);

        let descriptor_set_layout = Self::create_descriptor_set_layout(context);
        let pipeline_layout = Self::create_pipeline_layout(context, descriptor_set_layout);
        let pipeline =
            Self::create_graphics_pipeline(context, window, render_pass, pipeline_layout);

        // Command pool — no RESET flag needed since we record once
        let command_pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(context.graphics_queue_index as u32);
        let command_pool = context
            .logical_device
            .create_command_pool(&command_pool_info, None)
            .expect("Failed to create command pool");

        // Shared geometry
        let vertex_buffer = vulkan_buffer::VulkanBuffer::new(
            context,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            std::mem::size_of_val(&primitives::QUAD_VERTS),
        );
        vertex_buffer.Write(&primitives::QUAD_VERTS, context);

        let index_buffer = vulkan_buffer::VulkanBuffer::new(
            context,
            vk::BufferUsageFlags::INDEX_BUFFER,
            std::mem::size_of_val(&QUAD_INDICES),
        );
        index_buffer.Write(&QUAD_INDICES, context);

        // Default texture
        let default_texture = vulkan_texture::VulkanTexture::white_pixel(context);

        // Per-frame resources (double-buffered)
        let frames = std::array::from_fn(|_| {
            Self::create_frame_resources(context, descriptor_set_layout, &default_texture)
        });

        // Record one command buffer per swapchain image — these are static
        let command_buffers = Self::record_command_buffers(
            context,
            command_pool,
            pipeline,
            pipeline_layout,
            render_pass,
            &framebuffers,
            &frames,
            &vertex_buffer,
            &index_buffer,
        );

        // Semaphores
        let sem_info = vk::SemaphoreCreateInfo::builder();
        let image_available_semaphores = [
            context
                .logical_device
                .create_semaphore(&sem_info, None)
                .unwrap(),
            context
                .logical_device
                .create_semaphore(&sem_info, None)
                .unwrap(),
        ];
        let render_finished_semaphores = [
            context
                .logical_device
                .create_semaphore(&sem_info, None)
                .unwrap(),
            context
                .logical_device
                .create_semaphore(&sem_info, None)
                .unwrap(),
        ];

        Self {
            pipeline,
            pipeline_layout,
            render_pass,
            descriptor_set_layout,
            framebuffers,
            command_pool,
            command_buffers,
            vertex_buffer,
            index_buffer,
            frames,
            current_frame: 0,
            image_available_semaphores,
            render_finished_semaphores,
            default_texture,
        }
    }

    unsafe fn create_frame_resources(
        context: &vulkan_context::VulkanContext,
        descriptor_set_layout: vk::DescriptorSetLayout,
        default_texture: &vulkan_texture::VulkanTexture,
    ) -> FrameResources {
        let instance_buffer = vulkan_buffer::VulkanBuffer::new(
            context,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            std::mem::size_of::<InstanceData>() * MAX_SPRITES,
        );

        let indirect_buffer = vulkan_buffer::VulkanBuffer::new(
            context,
            vk::BufferUsageFlags::INDIRECT_BUFFER,
            std::mem::size_of::<DrawIndexedIndirectCommand>() * MAX_SPRITES,
        );

        let camera_buffer = vulkan_buffer::VulkanBuffer::new(
            context,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            std::mem::size_of::<CameraUbo>(),
        );

        // Fence (start signalled so we can wait on it immediately the first time)
        let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);
        let fence = context
            .logical_device
            .create_fence(&fence_info, None)
            .expect("Failed to create fence");

        // Descriptor set
        let descriptor_set = Self::create_descriptor_set_static(
            context,
            descriptor_set_layout,
            &camera_buffer,
            default_texture,
        );

        FrameResources {
            instance_buffer,
            indirect_buffer,
            camera_buffer,
            descriptor_set,
            fence,
        }
    }

    // -------------------------------------------------------------------
    // Command buffer recording (one-time at init)
    // -------------------------------------------------------------------

    /// Record one command buffer per swapchain image. Each command buffer
    /// references its corresponding framebuffer and the double-buffered
    /// frame resources. Since the indirect buffer controls what is actually
    /// drawn, these command buffers never need to be re-recorded.
    ///
    /// We record `framebuffers.len() * frames.len()` command buffers total:
    /// for each (swapchain_image, frame_index) pair we get a dedicated cmd.
    /// Index into `command_buffers` as: `image_index * 2 + frame_index`.
    unsafe fn record_command_buffers(
        context: &vulkan_context::VulkanContext,
        command_pool: vk::CommandPool,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        render_pass: vk::RenderPass,
        framebuffers: &[vk::Framebuffer],
        frames: &[FrameResources; 2],
        vertex_buffer: &vulkan_buffer::VulkanBuffer,
        index_buffer: &vulkan_buffer::VulkanBuffer,
    ) -> Vec<vk::CommandBuffer> {
        let total = framebuffers.len() * 2; // 2 for double-buffering

        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(total as u32);

        let command_buffers = context
            .logical_device
            .allocate_command_buffers(&alloc_info)
            .expect("Failed to allocate command buffers");

        for image_idx in 0..framebuffers.len() {
            for frame_idx in 0..2usize {
                let cmd_idx = image_idx * 2 + frame_idx;
                let cmd = command_buffers[cmd_idx];
                let frame = &frames[frame_idx];

                let begin_info = vk::CommandBufferBeginInfo::builder();
                context
                    .logical_device
                    .begin_command_buffer(cmd, &begin_info)
                    .expect("Failed to begin command buffer");

                let clear_values = &[vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                }];

                let render_pass_info = vk::RenderPassBeginInfo::builder()
                    .render_pass(render_pass)
                    .framebuffer(framebuffers[image_idx])
                    .render_area(
                        vk::Rect2D::builder()
                            .offset(vk::Offset2D { x: 0, y: 0 })
                            .extent(context.swapchain_extent),
                    )
                    .clear_values(clear_values);

                context.logical_device.cmd_begin_render_pass(
                    cmd,
                    &render_pass_info,
                    vk::SubpassContents::INLINE,
                );

                context.logical_device.cmd_bind_pipeline(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline,
                );

                context.logical_device.cmd_bind_descriptor_sets(
                    cmd,
                    vk::PipelineBindPoint::GRAPHICS,
                    pipeline_layout,
                    0,
                    &[frame.descriptor_set],
                    &[],
                );

                // Bind vertex buffer (binding 0) and instance buffer (binding 1)
                context.logical_device.cmd_bind_vertex_buffers(
                    cmd,
                    0,
                    &[vertex_buffer.buffer, frame.instance_buffer.buffer],
                    &[0, 0],
                );

                context.logical_device.cmd_bind_index_buffer(
                    cmd,
                    index_buffer.buffer,
                    0,
                    vk::IndexType::UINT16,
                );

                // Draw ALL MAX_SPRITES. Unused slots have instance_count=0
                // in the indirect buffer, so the GPU skips them for free.
                context.logical_device.cmd_draw_indexed_indirect(
                    cmd,
                    frame.indirect_buffer.buffer,
                    0,
                    MAX_SPRITES as u32,
                    std::mem::size_of::<DrawIndexedIndirectCommand>() as u32,
                );

                context.logical_device.cmd_end_render_pass(cmd);

                context
                    .logical_device
                    .end_command_buffer(cmd)
                    .expect("Failed to end command buffer");
            }
        }

        command_buffers
    }

    // -------------------------------------------------------------------
    // Descriptor set
    // -------------------------------------------------------------------

    unsafe fn create_descriptor_set_layout(
        context: &vulkan_context::VulkanContext,
    ) -> vk::DescriptorSetLayout {
        let bindings = [
            // binding 0: camera UBO
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
            // binding 1: texture atlas sampler
            vk::DescriptorSetLayoutBinding::builder()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build(),
        ];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);
        context
            .logical_device
            .create_descriptor_set_layout(&layout_info, None)
            .expect("Failed to create descriptor set layout")
    }

    unsafe fn create_descriptor_set_static(
        context: &vulkan_context::VulkanContext,
        layout: vk::DescriptorSetLayout,
        camera_buffer: &vulkan_buffer::VulkanBuffer,
        texture: &vulkan_texture::VulkanTexture,
    ) -> vk::DescriptorSet {
        let layouts = [layout];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(context.descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_set = context
            .logical_device
            .allocate_descriptor_sets(&alloc_info)
            .expect("Failed to allocate descriptor set")[0];

        // Write UBO
        let buffer_info = [vk::DescriptorBufferInfo::builder()
            .buffer(camera_buffer.buffer)
            .offset(0)
            .range(std::mem::size_of::<CameraUbo>() as u64)
            .build()];

        let ubo_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&buffer_info);

        // Write sampler
        let image_info = [vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(texture.view)
            .sampler(texture.sampler)
            .build()];

        let sampler_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_info);

        context.logical_device.update_descriptor_sets(
            &[*ubo_write, *sampler_write],
            &[] as &[vk::CopyDescriptorSet],
        );

        descriptor_set
    }

    /// Update the texture atlas binding on a frame's descriptor set.
    pub unsafe fn update_texture(
        &self,
        context: &vulkan_context::VulkanContext,
        frame_index: usize,
        texture: &vulkan_texture::VulkanTexture,
    ) {
        let image_info = [vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(texture.view)
            .sampler(texture.sampler)
            .build()];

        let write = vk::WriteDescriptorSet::builder()
            .dst_set(self.frames[frame_index].descriptor_set)
            .dst_binding(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(&image_info);

        context
            .logical_device
            .update_descriptor_sets(&[*write], &[] as &[vk::CopyDescriptorSet]);
    }

    // -------------------------------------------------------------------
    // Render pass
    // -------------------------------------------------------------------

    unsafe fn create_render_pass(context: &vulkan_context::VulkanContext) -> vk::RenderPass {
        let color_attachment = vk::AttachmentDescription::builder()
            .format(context.surface_format)
            .samples(vk::SampleCountFlags::_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR);

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

        let color_attachments = &[color_attachment_ref];
        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(color_attachments);

        let dependency = vk::SubpassDependency::builder()
            .src_subpass(vk::SUBPASS_EXTERNAL)
            .dst_subpass(0)
            .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
            .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE);

        let attachments = &[color_attachment];
        let subpasses = &[subpass];
        let dependencies = &[dependency];
        let info = vk::RenderPassCreateInfo::builder()
            .attachments(attachments)
            .subpasses(subpasses)
            .dependencies(dependencies);

        context
            .logical_device
            .create_render_pass(&info, None)
            .expect("Failed to create render pass")
    }

    unsafe fn create_framebuffers(
        context: &vulkan_context::VulkanContext,
        render_pass: vk::RenderPass,
    ) -> Vec<vk::Framebuffer> {
        context
            .swapchain_image_views
            .iter()
            .map(|view| {
                let attachments = &[*view];
                let info = vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(attachments)
                    .width(context.swapchain_extent.width)
                    .height(context.swapchain_extent.height)
                    .layers(1);
                context
                    .logical_device
                    .create_framebuffer(&info, None)
                    .expect("Failed to create framebuffer")
            })
            .collect()
    }

    // -------------------------------------------------------------------
    // Pipeline
    // -------------------------------------------------------------------

    unsafe fn create_pipeline_layout(
        context: &vulkan_context::VulkanContext,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> vk::PipelineLayout {
        let layouts = [descriptor_set_layout];
        let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&layouts);
        context
            .logical_device
            .create_pipeline_layout(&layout_info, None)
            .expect("Failed to create pipeline layout")
    }

    unsafe fn create_graphics_pipeline(
        context: &vulkan_context::VulkanContext,
        window: &winit::window::Window,
        render_pass: vk::RenderPass,
        pipeline_layout: vk::PipelineLayout,
    ) -> vk::Pipeline {
        let vert_module = context.create_shader_module(rust_vertex_shader::VERT_SHADER);
        let frag_module = context.create_shader_module(rust_fragment_shader::FRAG_SHADER);

        let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(b"main\0");

        let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(b"main\0");
 
        let binding_config = VertexBindingConfig::new();
        let bindings = binding_config.bindings();
        let attributes = binding_config.attributes();

        let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&bindings)
            .vertex_attribute_descriptions(&attributes);

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
            .cull_mode(vk::CullModeFlags::NONE) // No culling for 2D sprites
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::_1);

        // Alpha blending for sprite transparency
        let attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::all())
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);

        let attachments = &[attachment];
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(attachments)
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let stages = &[vert_stage, frag_stage];
        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(stages)
            .vertex_input_state(&vertex_input_state)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .color_blend_state(&color_blend_state)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        let pipeline = context
            .logical_device
            .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
            .expect("Failed to create graphics pipeline")
            .0[0];

        // Shader modules can be destroyed after pipeline creation
        context
            .logical_device
            .destroy_shader_module(vert_module, None);
        context
            .logical_device
            .destroy_shader_module(frag_module, None);

        pipeline
    }

    // -------------------------------------------------------------------
    // Draw implementation
    // -------------------------------------------------------------------

    unsafe fn draw_impl(
        &mut self,
        context: &vulkan_context::VulkanContext,
        sprites: &[SpriteInstance],
    ) {
        let frame_idx = self.current_frame;
        let frame = &self.frames[frame_idx];

        // Wait for this frame's previous submission to complete
        context
            .logical_device
            .wait_for_fences(&[frame.fence], true, u64::MAX)
            .expect("Failed to wait for fence");
        context
            .logical_device
            .reset_fences(&[frame.fence])
            .expect("Failed to reset fence");

        // Acquire the next swapchain image
        let (image_index, _) = context
            .logical_device
            .acquire_next_image_khr(
                context.swapchain,
                u64::MAX,
                self.image_available_semaphores[frame_idx],
                vk::Fence::null(),
            )
            .expect("Failed to acquire next image");

        // -- CPU-side buffer update (no command buffer recording!) --
        let sprite_count = sprites.len().min(MAX_SPRITES);

        // Build the full indirect command array. Active sprites get
        // instance_count=1, unused slots get instance_count=0 so the
        // GPU skips them (the pre-recorded command buffer always
        // dispatches MAX_SPRITES indirect draws).
        let mut instance_data: Vec<InstanceData> = Vec::with_capacity(MAX_SPRITES);
        let mut indirect_cmds: Vec<DrawIndexedIndirectCommand> = Vec::with_capacity(MAX_SPRITES);

        for i in 0..sprite_count {
            let sprite = &sprites[i];

            // Convert nalgebra Matrix4 to column-major [[f32;4];4]
            let m = &sprite.transform;
            let model: [[f32; 4]; 4] = [
                [m[(0, 0)], m[(1, 0)], m[(2, 0)], m[(3, 0)]],
                [m[(0, 1)], m[(1, 1)], m[(2, 1)], m[(3, 1)]],
                [m[(0, 2)], m[(1, 2)], m[(2, 2)], m[(3, 2)]],
                [m[(0, 3)], m[(1, 3)], m[(2, 3)], m[(3, 3)]],
            ];

            instance_data.push(InstanceData {
                model,
                atlas_rect: sprite.atlas_rect,
            });

            indirect_cmds.push(DrawIndexedIndirectCommand {
                index_count: 6,
                instance_count: 1,
                first_index: 0,
                vertex_offset: 0,
                first_instance: i as u32,
            });
        }

        // Fill remaining slots with no-op draws (instance_count = 0)
        for i in sprite_count..MAX_SPRITES {
            instance_data.push(InstanceData::default());
            indirect_cmds.push(DrawIndexedIndirectCommand {
                index_count: 6,
                instance_count: 0, // GPU skips this draw
                first_index: 0,
                vertex_offset: 0,
                first_instance: i as u32,
            });
        }

        // Upload buffers
        // Upload buffers. We pass a reference to the first element so VulkanBuffer::Write
        // gets a raw pointer to the heap array, not the stack Vec metadata struct!
        frame.instance_buffer.Write(&instance_data[0], context);
        frame.indirect_buffer.Write(&indirect_cmds[0], context);

        // Update camera UBO (orthographic projection for 2D)
        let w = context.swapchain_extent.width as f32;
        let h = context.swapchain_extent.height as f32;
        let camera = CameraUbo {
            view: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            projection: ortho_projection(w, h),
        };
        frame.camera_buffer.Write(&camera, context);

        // -- Submit the pre-recorded command buffer --
        let cmd_idx = image_index as usize * 2 + frame_idx;
        let cmd = self.command_buffers[cmd_idx];

        let wait_semaphores = &[self.image_available_semaphores[frame_idx]];
        let signal_semaphores = &[self.render_finished_semaphores[frame_idx]];
        let command_buffers = &[cmd];

        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(command_buffers)
            .signal_semaphores(signal_semaphores);

        context
            .logical_device
            .queue_submit(context.graphics_queue, &[submit_info], frame.fence)
            .expect("Failed to submit draw command");

        // -- Present --
        let swapchains = &[context.swapchain];
        let image_indices = &[image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(signal_semaphores)
            .swapchains(swapchains)
            .image_indices(image_indices);

        context
            .logical_device
            .queue_present_khr(context.present_queue, &present_info)
            .expect("Failed to present");

        context
            .logical_device
            .queue_wait_idle(context.present_queue)
            .expect("Failed to wait queue idle");

        // Advance frame index
        self.current_frame = (self.current_frame + 1) % 2;
    }
}

// -----------------------------------------------------------------------
// Utility
// -----------------------------------------------------------------------

/// Simple orthographic projection for 2D rendering.
/// Maps screen coordinates where (0,0) is top-left.
fn ortho_projection(width: f32, height: f32) -> [[f32; 4]; 4] {
    let l = 0.0_f32;
    let r = width;
    let t = 0.0_f32;
    let b = height;
    let n = -1.0_f32;
    let f = 1.0_f32;

    // Column-major layout matching GLSL mat4
    [
        [2.0 / (r - l), 0.0, 0.0, 0.0],
        [0.0, 2.0 / (b - t), 0.0, 0.0],
        [0.0, 0.0, 1.0 / (f - n), 0.0],
        [-(r + l) / (r - l), -(b + t) / (b - t), -n / (f - n), 1.0],
    ]
}
