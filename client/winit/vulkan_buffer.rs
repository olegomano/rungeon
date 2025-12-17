use std::ptr::copy_nonoverlapping as memcpy;
use vulkanalia::bytecode::Bytecode;
use vulkanalia::loader::{LibloadingLoader, LIBRARY};
use vulkanalia::prelude::v1_0::*;
use vulkanalia::vk::ExtDebugUtilsExtension;
use vulkanalia::vk::KhrSurfaceExtension;
use vulkanalia::vk::KhrSwapchainExtension;

extern crate vulkan_context;
use vulkan_context::VulkanContext;

/*
* Buffer is like a typed pointer, ie the data descriptor
* DeviceMemory is the actual pointer to the data
*/
#[derive(Debug)]
pub struct VulkanBuffer {
    pub buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub size: usize,
}

impl VulkanBuffer {
    pub unsafe fn new(
        context: &VulkanContext,
        buffer_type: vk::BufferUsageFlags,
        size: usize,
    ) -> VulkanBuffer {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size as u64)
            .usage(buffer_type)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

        let buffer = context
            .logical_device
            .create_buffer(&buffer_info, None)
            .expect("");

        let requirements = context
            .logical_device
            .get_buffer_memory_requirements(buffer);

        let memory_properties = context
            .instance
            .get_physical_device_memory_properties(context.physical_device);

        let memory_type_index = (0..memory_properties.memory_type_count)
            .find(|i| {
                let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
                let memory_type = memory_properties.memory_types[*i as usize];
                suitable
                    && memory_type.property_flags.contains(
                        vk::MemoryPropertyFlags::HOST_COHERENT
                            | vk::MemoryPropertyFlags::HOST_VISIBLE,
                    )
            })
            .expect("");

        let memory_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type_index);

        let vertex_buffer_memory = context
            .logical_device
            .allocate_memory(&memory_info, None)
            .expect("");
        context
            .logical_device
            .bind_buffer_memory(buffer, vertex_buffer_memory, 0)
            .expect("");

        return VulkanBuffer {
            buffer: buffer,
            vertex_buffer_memory: vertex_buffer_memory,
            size: size,
        };
    }

    pub unsafe fn Write<T>(&self, data: &T, context: &VulkanContext) {
        let memory = context
            .logical_device
            .map_memory(
                self.vertex_buffer_memory,
                0,
                self.size as u64,
                vk::MemoryMapFlags::empty(),
            )
            .expect("");

        memcpy(data as *const T as *const u8, memory.cast(), self.size);

        context
            .logical_device
            .unmap_memory(self.vertex_buffer_memory);
    }
}

#[derive(Debug)]
pub struct VulkanObject<T> {
    pub buffer: VulkanBuffer,
    pub data: T,
}

impl<T: Default> VulkanObject<T> {
    pub fn new(context: &VulkanContext) -> Self {
        unsafe {
            return Self {
                buffer: VulkanBuffer::new(
                    context,
                    vk::BufferUsageFlags::UNIFORM_BUFFER,
                    std::mem::size_of::<T>(),
                ),
                data: T::default(),
            };
        }
    }

    //write out the current state of data to the gpu
    pub fn Sync(&self, context: &VulkanContext) {
        unsafe {
            self.buffer.Write(&self.data, context);
        }
    }
}
