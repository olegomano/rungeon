use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::prelude::v1_0::*;

extern crate vulkan_context;
use vulkan_context::VulkanContext;

/// A GPU-resident 2D texture with its associated image view and sampler.
///
/// The texture is created in a SHADER_READ_ONLY_OPTIMAL layout suitable for
/// sampling in fragment shaders. Image data is uploaded through a staging
/// buffer, then transitioned to the optimal layout via a one-shot command
/// buffer.
#[derive(Debug)]
pub struct VulkanTexture {
    pub image: vk::Image,
    pub memory: vk::DeviceMemory,
    pub view: vk::ImageView,
    pub sampler: vk::Sampler,
    pub width: u32,
    pub height: u32,
    pub format: vk::Format,
}

impl VulkanTexture {
    /// Create a texture from raw RGBA8 pixel data.
    ///
    /// `pixels` must contain exactly `width * height * 4` bytes.
    pub unsafe fn from_rgba8(
        context: &VulkanContext,
        width: u32,
        height: u32,
        pixels: &[u8],
    ) -> Self {
        let format = vk::Format::R8G8B8A8_SRGB;
        let image_size = (width * height * 4) as u64;

        assert_eq!(
            pixels.len() as u64,
            image_size,
            "Pixel data size mismatch"
        );

        // --- Staging buffer ---
        let staging_buffer_info = vk::BufferCreateInfo::builder()
            .size(image_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let staging_buffer = context
            .logical_device
            .create_buffer(&staging_buffer_info, None)
            .expect("Failed to create staging buffer");

        let staging_reqs = context
            .logical_device
            .get_buffer_memory_requirements(staging_buffer);

        let staging_mem_index =
            find_memory_type(context, staging_reqs.memory_type_bits, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);

        let staging_alloc = vk::MemoryAllocateInfo::builder()
            .allocation_size(staging_reqs.size)
            .memory_type_index(staging_mem_index);

        let staging_memory = context
            .logical_device
            .allocate_memory(&staging_alloc, None)
            .expect("Failed to allocate staging memory");

        context
            .logical_device
            .bind_buffer_memory(staging_buffer, staging_memory, 0)
            .expect("Failed to bind staging memory");

        // Copy pixel data to staging buffer
        let data = context
            .logical_device
            .map_memory(staging_memory, 0, image_size, vk::MemoryMapFlags::empty())
            .expect("Failed to map staging memory");
        memcpy(pixels.as_ptr(), data as *mut u8, pixels.len());
        context.logical_device.unmap_memory(staging_memory);

        // --- Image ---
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::_2D)
            .extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .samples(vk::SampleCountFlags::_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image = context
            .logical_device
            .create_image(&image_info, None)
            .expect("Failed to create image");

        let image_reqs = context.logical_device.get_image_memory_requirements(image);

        let image_mem_index = find_memory_type(
            context,
            image_reqs.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let image_alloc = vk::MemoryAllocateInfo::builder()
            .allocation_size(image_reqs.size)
            .memory_type_index(image_mem_index);

        let image_memory = context
            .logical_device
            .allocate_memory(&image_alloc, None)
            .expect("Failed to allocate image memory");

        context
            .logical_device
            .bind_image_memory(image, image_memory, 0)
            .expect("Failed to bind image memory");

        // --- Transition + Copy via one-shot command buffer ---
        let cmd = begin_single_time_commands(context);

        // Transition UNDEFINED -> TRANSFER_DST_OPTIMAL
        transition_image_layout(
            context,
            cmd,
            image,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        );

        // Copy buffer to image
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });

        context.logical_device.cmd_copy_buffer_to_image(
            cmd,
            staging_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );

        // Transition TRANSFER_DST_OPTIMAL -> SHADER_READ_ONLY_OPTIMAL
        transition_image_layout(
            context,
            cmd,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        );

        end_single_time_commands(context, cmd);

        // Cleanup staging
        context.logical_device.destroy_buffer(staging_buffer, None);
        context.logical_device.free_memory(staging_memory, None);

        // --- Image view ---
        let view_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::_2D)
            .format(format)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(1)
                    .build(),
            );

        let view = context
            .logical_device
            .create_image_view(&view_info, None)
            .expect("Failed to create image view");

        // --- Sampler ---
        let sampler_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::NEAREST)
            .min_filter(vk::Filter::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .anisotropy_enable(false)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
            .mip_lod_bias(0.0)
            .min_lod(0.0)
            .max_lod(0.0);

        let sampler = context
            .logical_device
            .create_sampler(&sampler_info, None)
            .expect("Failed to create sampler");

        Self {
            image,
            memory: image_memory,
            view,
            sampler,
            width,
            height,
            format,
        }
    }

    /// Create a 1x1 white placeholder texture.
    pub unsafe fn white_pixel(context: &VulkanContext) -> Self {
        Self::from_rgba8(context, 1, 1, &[255, 255, 255, 255])
    }
}

// ---------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------

unsafe fn find_memory_type(
    context: &VulkanContext,
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
) -> u32 {
    let memory_properties = context
        .instance
        .get_physical_device_memory_properties(context.physical_device);

    (0..memory_properties.memory_type_count)
        .find(|i| {
            let suitable = (type_filter & (1 << i)) != 0;
            let mem_type = memory_properties.memory_types[*i as usize];
            suitable && mem_type.property_flags.contains(properties)
        })
        .expect("Failed to find suitable memory type")
}

unsafe fn begin_single_time_commands(context: &VulkanContext) -> vk::CommandBuffer {
    let pool_info = vk::CommandPoolCreateInfo::builder()
        .queue_family_index(context.graphics_queue_index as u32)
        .flags(vk::CommandPoolCreateFlags::TRANSIENT);

    let pool = context
        .logical_device
        .create_command_pool(&pool_info, None)
        .expect("Failed to create transient command pool");

    let alloc_info = vk::CommandBufferAllocateInfo::builder()
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(pool)
        .command_buffer_count(1);

    let cmd = context
        .logical_device
        .allocate_command_buffers(&alloc_info)
        .expect("Failed to allocate command buffer")[0];

    let begin_info =
        vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    context
        .logical_device
        .begin_command_buffer(cmd, &begin_info)
        .expect("Failed to begin command buffer");

    cmd
}

unsafe fn end_single_time_commands(context: &VulkanContext, cmd: vk::CommandBuffer) {
    context
        .logical_device
        .end_command_buffer(cmd)
        .expect("Failed to end command buffer");

    let cmd_buffers = [cmd];
    let submit_info = vk::SubmitInfo::builder().command_buffers(&cmd_buffers);

    context
        .logical_device
        .queue_submit(context.graphics_queue, &[submit_info], vk::Fence::null())
        .expect("Failed to submit command buffer");

    context
        .logical_device
        .queue_wait_idle(context.graphics_queue)
        .expect("Failed to wait for queue idle");
}

unsafe fn transition_image_layout(
    context: &VulkanContext,
    cmd: vk::CommandBuffer,
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) {
    let (src_access, dst_access, src_stage, dst_stage) = match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
            vk::AccessFlags::empty(),
            vk::AccessFlags::TRANSFER_WRITE,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
        ),
        (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
            vk::AccessFlags::TRANSFER_WRITE,
            vk::AccessFlags::SHADER_READ,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::FRAGMENT_SHADER,
        ),
        _ => panic!("Unsupported layout transition"),
    };

    let barrier = vk::ImageMemoryBarrier::builder()
        .old_layout(old_layout)
        .new_layout(new_layout)
        .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
        .image(image)
        .subresource_range(
            vk::ImageSubresourceRange::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .base_mip_level(0)
                .level_count(1)
                .base_array_layer(0)
                .layer_count(1)
                .build(),
        )
        .src_access_mask(src_access)
        .dst_access_mask(dst_access);

    context.logical_device.cmd_pipeline_barrier(
        cmd,
        src_stage,
        dst_stage,
        vk::DependencyFlags::empty(),
        &[] as &[vk::MemoryBarrier],
        &[] as &[vk::BufferMemoryBarrier],
        &[barrier],
    );
}
