use core::ops::Mul;

use bytemuck::NoUninit;

#[inline]
pub(crate) fn upload_buffer<T: NoUninit>(queue: &wgpu::Queue, buffer: &wgpu::Buffer, data: &[T]) {
    let byte_size = std::mem::size_of_val(data);
    if byte_size == 0 {
        return;
    }

    let mut view = queue
        .write_buffer_with(buffer, 0, wgpu::BufferSize::new(byte_size as u64).unwrap())
        .unwrap();

    view.clone_from_slice(bytemuck::cast_slice(data));
}

#[inline]
pub(crate) fn upload_buffer_or_recreate<T: NoUninit>(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &mut wgpu::Buffer, data: &[T]) {
    let byte_size = std::mem::size_of_val(data);
    if byte_size == 0 {
        return;
    }

    // already unpadded
    let unpadded_size = byte_size as wgpu::BufferAddress;

    if byte_size > buffer.size() as usize {
        // Copied from wgpu code.
        let size = {
            // Valid vulkan usage is
            // 1. buffer size must be a multiple of COPY_BUFFER_ALIGNMENT.
            // 2. buffer size must be greater than 0.
            // Therefore we round the value up to the nearest multiple, and ensure it's at least COPY_BUFFER_ALIGNMENT.
            let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;

            ((unpadded_size + align_mask) & !align_mask).max(wgpu::COPY_BUFFER_ALIGNMENT)
        };

        *buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size,
            usage: buffer.usage(),
            mapped_at_creation: false,
        });
    }

    upload_buffer(queue, buffer, data);
}

#[inline]
pub fn copy_texture_size(size: wgpu::Extent3d, format: wgpu::TextureFormat, pixel_byte_size: u32) -> wgpu::Extent3d {
    let size = size.physical_size(format);

    let padded_width = size
        .width
        .div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        .mul(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);

    wgpu::Extent3d {
        width: padded_width * pixel_byte_size,
        ..size
    }
}

#[inline]
pub fn pixel_data_buffer_size(size: wgpu::Extent3d) -> u64 {
    ((size.width * size.height) as u64)
        .div_ceil(wgpu::COPY_BUFFER_ALIGNMENT)
        .mul(wgpu::COPY_BUFFER_ALIGNMENT)
}
