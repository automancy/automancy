mod game;
mod shaders;
mod util;

pub use game::*;
pub use shaders::*;
pub use util::*;

#[derive(Debug)]
pub enum BufferInitArg<'a> {
    Init {
        size: u64,
        cpu_buffer: &'a [(wgpu::BufferAddress, &'a [u8])],
    },
    RecreateIfNeeded {
        min_size: u64,
        buffer: wgpu::Buffer,
        cpu_buffer: &'a [(wgpu::BufferAddress, &'a [u8])],
    },
    Reuse(wgpu::Buffer),
}

impl<'a> BufferInitArg<'a> {
    pub const fn init<const SIZE: usize>() -> BufferInitArg<'a> {
        BufferInitArg::Init {
            size: SIZE as u64,
            cpu_buffer: &[(0, &[0; SIZE])],
        }
    }
}

impl Default for BufferInitArg<'_> {
    fn default() -> Self {
        BufferInitArg::init::<512>()
    }
}

#[inline]
pub(crate) fn new_buffer_size(gpu_size: u64, cpu_size: u64) -> u64 {
    gpu_size.max(cpu_size + (cpu_size >> 1))
}
