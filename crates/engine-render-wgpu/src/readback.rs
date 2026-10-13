//! Bounded WGPU texture readback with complete padded-row stripping.

use std::sync::mpsc;
use std::time::Duration;

use crate::targets::{FrameTargets, TargetError, TargetSize, copy_bytes_per_row, packed_byte_len};

pub(crate) struct ReadbackBuffer {
    pub(crate) buffer: wgpu::Buffer,
    pub(crate) bytes_per_row: u32,
}

impl ReadbackBuffer {
    pub(crate) fn create(device: &wgpu::Device, size: TargetSize) -> Result<Self, ReadbackError> {
        let bytes_per_row = copy_bytes_per_row(size).map_err(ReadbackError::Target)?;
        let allocation_size = u64::from(bytes_per_row)
            .checked_mul(u64::from(size.height()))
            .ok_or(ReadbackError::SizeOverflow)?;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("openunity-frame-readback"),
            size: allocation_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Ok(Self {
            buffer,
            bytes_per_row,
        })
    }

    pub(crate) fn encode_copy(&self, encoder: &mut wgpu::CommandEncoder, targets: &FrameTargets) {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &targets.color,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bytes_per_row),
                    rows_per_image: Some(targets.size.height()),
                },
            },
            wgpu::Extent3d {
                width: targets.size.width(),
                height: targets.size.height(),
                depth_or_array_layers: 1,
            },
        );
    }
}

pub(crate) fn map_rgba8(
    device: &wgpu::Device,
    readback: &ReadbackBuffer,
    submission: wgpu::SubmissionIndex,
    size: TargetSize,
    timeout: Duration,
) -> Result<Vec<u8>, ReadbackError> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission.clone()),
            timeout: Some(timeout),
        })
        .map_err(ReadbackError::from_poll_error)?;

    let slice = readback.buffer.slice(..);
    let (sender, receiver) = mpsc::sync_channel(1);
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    if let Err(error) = device.poll(wgpu::PollType::Wait {
        submission_index: Some(submission),
        timeout: Some(timeout),
    }) {
        readback.buffer.unmap();
        return Err(ReadbackError::from_poll_error(error));
    }
    let map_result = match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            readback.buffer.unmap();
            return Err(ReadbackError::MapTimeout);
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            readback.buffer.unmap();
            return Err(ReadbackError::MapCallback);
        }
    };
    map_result.map_err(ReadbackError::Map)?;

    let _unmap = UnmapOnDrop {
        buffer: &readback.buffer,
    };
    copy_tightly_packed_rows(&readback.buffer, size, readback.bytes_per_row)
}

struct UnmapOnDrop<'a> {
    buffer: &'a wgpu::Buffer,
}

impl Drop for UnmapOnDrop<'_> {
    fn drop(&mut self) {
        self.buffer.unmap();
    }
}

fn copy_tightly_packed_rows(
    buffer: &wgpu::Buffer,
    size: TargetSize,
    bytes_per_row: u32,
) -> Result<Vec<u8>, ReadbackError> {
    let packed_row =
        usize::try_from(u64::from(size.width()) * 4).map_err(|_| ReadbackError::SizeOverflow)?;
    let row_stride = usize::try_from(bytes_per_row).map_err(|_| ReadbackError::SizeOverflow)?;
    let output_len = packed_byte_len(size).map_err(ReadbackError::Target)?;
    let mapped = buffer.slice(..).get_mapped_range();
    let mut output = Vec::with_capacity(output_len);
    for row in 0..size.height() as usize {
        let start = row
            .checked_mul(row_stride)
            .ok_or(ReadbackError::SizeOverflow)?;
        let end = start
            .checked_add(packed_row)
            .ok_or(ReadbackError::SizeOverflow)?;
        let source = mapped.get(start..end).ok_or(ReadbackError::ShortMapping)?;
        output.extend_from_slice(source);
    }
    drop(mapped);
    Ok(output)
}

#[derive(Debug)]
pub(crate) enum ReadbackError {
    Target(TargetError),
    SizeOverflow,
    Poll,
    SubmissionTimeout,
    MapTimeout,
    MapCallback,
    Map(wgpu::BufferAsyncError),
    ShortMapping,
}

impl ReadbackError {
    fn from_poll_error(error: wgpu::PollError) -> Self {
        if matches!(error, wgpu::PollError::Timeout) {
            Self::SubmissionTimeout
        } else {
            Self::Poll
        }
    }
}
