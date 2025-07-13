use bytemuck::NoUninit;

#[inline]
pub(crate) fn upload_buffer<T: NoUninit>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    data: &[T],
) {
    let byte_size = std::mem::size_of_val(data);
    // already unpadded
    let unpadded_size = byte_size as wgpu::BufferAddress;

    if byte_size > buffer.size() as usize {
        let unpadded_size = unpadded_size * 2;

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

    let mut view = queue
        .write_buffer_with(buffer, 0, wgpu::BufferSize::new(byte_size as u64).unwrap())
        .unwrap();

    view.clone_from_slice(bytemuck::cast_slice(data));
}
