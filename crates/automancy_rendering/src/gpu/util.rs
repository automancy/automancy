// Copied from wgpu code.
#[inline(always)]
pub fn align_u64<const ALIGNMENT: u64>(size: u64) -> u64 {
    let align_mask = ALIGNMENT - 1;

    ((size + align_mask) & !align_mask).max(ALIGNMENT)
}

// Copied from wgpu code.
#[inline(always)]
pub fn align_u32<const ALIGNMENT: u32>(size: u32) -> u32 {
    let align_mask = ALIGNMENT - 1;

    ((size + align_mask) & !align_mask).max(ALIGNMENT)
}

#[inline]
pub fn copy_texture_size(size: wgpu::Extent3d, format: wgpu::TextureFormat, pixel_byte_size: u32) -> wgpu::Extent3d {
    let size = size.physical_size(format);

    let padded_width = align_u32::<{ wgpu::COPY_BYTES_PER_ROW_ALIGNMENT }>(size.width);

    wgpu::Extent3d {
        width: padded_width * pixel_byte_size,
        ..size
    }
}

#[inline]
pub fn pixel_data_buffer_size(size: wgpu::Extent3d) -> u64 {
    align_u64::<{ wgpu::COPY_BUFFER_ALIGNMENT }>((size.width * size.height) as u64)
}

#[cfg_attr(feature = "profile", profiling::function)]
#[inline]
#[track_caller]
pub fn init_buffer(device: &wgpu::Device, desc: &wgpu::BufferDescriptor, data: &[(wgpu::BufferAddress, &[u8])]) -> wgpu::Buffer {
    if desc.size == 0 {
        return device.create_buffer(&wgpu::BufferDescriptor {
            size: 0,
            mapped_at_creation: false,
            ..*desc
        });
    }

    // already unpadded
    let unpadded_size = desc.size as wgpu::BufferAddress;
    let size = align_u64::<{ wgpu::COPY_BUFFER_ALIGNMENT }>(unpadded_size);

    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        size,
        mapped_at_creation: true,
        ..*desc
    });
    for &(offset, slice) in data {
        if slice.is_empty() {
            continue;
        }

        buffer
            .get_mapped_range_mut(offset..(offset + slice.len() as u64))
            .copy_from_slice(slice);
    }
    buffer.unmap();

    buffer
}

#[cfg_attr(feature = "profile", profiling::function)]
#[inline]
#[track_caller]
pub fn upload_buffer(queue: &wgpu::Queue, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress, data: &[u8]) {
    if data.is_empty() {
        return;
    }

    let mut view = queue
        .write_buffer_with(buffer, offset, wgpu::BufferSize::new(data.len() as u64).unwrap())
        .unwrap();

    view.copy_from_slice(data);
}

#[cfg_attr(feature = "profile", profiling::function)]
#[inline]
#[track_caller]
pub fn upload_buffer_or_recreate(device: &wgpu::Device, queue: &wgpu::Queue, buffer: &mut wgpu::Buffer, data: &[u8]) {
    if !data.is_empty() && data.len() <= buffer.size() as usize {
        upload_buffer(queue, buffer, 0, data);
    } else {
        *buffer = init_buffer(
            device,
            &wgpu::BufferDescriptor {
                label: None,
                size: data.len() as u64,
                usage: buffer.usage(),
                mapped_at_creation: false,
            },
            &[(0, data)],
        )
    }
}
