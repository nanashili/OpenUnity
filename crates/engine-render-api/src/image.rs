//! Backend-neutral checked image and receipt values.

use crate::{RenderError, RenderFrame, TargetSize};

/// Explicit adapter selection requested by the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterPolicy {
    /// Require an adapter classified as hardware.
    Hardware,
    /// Require an explicitly selected software/fallback adapter.
    Software,
}

/// The first slice's concrete render target format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TargetFormat {
    /// Four unorm bytes in RGBA channel order.
    Rgba8Unorm,
}

/// Identity copied from the backend's adapter info without API-side invention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterIdentity {
    backend: Box<str>,
    adapter_name: Box<str>,
    vendor: Box<str>,
    device: Box<str>,
    driver: Box<str>,
    driver_info: Box<str>,
    device_type: Box<str>,
}

impl AdapterIdentity {
    /// Retain the exact identity strings supplied by the concrete backend.
    pub fn new(
        backend: impl Into<Box<str>>,
        adapter_name: impl Into<Box<str>>,
        vendor: impl Into<Box<str>>,
        device: impl Into<Box<str>>,
        driver: impl Into<Box<str>>,
        driver_info: impl Into<Box<str>>,
        device_type: impl Into<Box<str>>,
    ) -> Self {
        Self {
            backend: backend.into(),
            adapter_name: adapter_name.into(),
            vendor: vendor.into(),
            device: device.into(),
            driver: driver.into(),
            driver_info: driver_info.into(),
            device_type: device_type.into(),
        }
    }

    /// Return the backend identity string supplied by the adapter.
    pub fn backend(&self) -> &str {
        &self.backend
    }
    /// Return the adapter name supplied by the adapter.
    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }
    /// Return the vendor supplied by the adapter.
    pub fn vendor(&self) -> &str {
        &self.vendor
    }
    /// Return the device string supplied by the adapter.
    pub fn device(&self) -> &str {
        &self.device
    }
    /// Return the driver string supplied by the adapter.
    pub fn driver(&self) -> &str {
        &self.driver
    }
    /// Return the driver information supplied by the adapter.
    pub fn driver_info(&self) -> &str {
        &self.driver_info
    }
    /// Return the adapter device type supplied by the adapter.
    pub fn device_type(&self) -> &str {
        &self.device_type
    }
}

/// Facts reported after a backend completes the frame submission and readback.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BackendCompletion {
    draw_count: usize,
    triangle_count: usize,
    readback_complete: bool,
}

impl BackendCompletion {
    /// Group completion facts for one rendered frame.
    pub const fn new(draw_count: usize, triangle_count: usize, readback_complete: bool) -> Self {
        Self {
            draw_count,
            triangle_count,
            readback_complete,
        }
    }
}

/// Backend identity and completion facts retained with a successful image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendReceipt {
    requested_policy: AdapterPolicy,
    adapter: AdapterIdentity,
    target_format: TargetFormat,
    target_size: TargetSize,
    frame_number: u64,
    draw_count: usize,
    triangle_count: usize,
    readback_complete: bool,
}

impl BackendReceipt {
    /// Create a receipt from facts reported by the concrete backend.
    pub fn new(
        requested_policy: AdapterPolicy,
        adapter: AdapterIdentity,
        target_format: TargetFormat,
        frame: &RenderFrame<'_>,
        completion: BackendCompletion,
    ) -> Self {
        Self {
            requested_policy,
            adapter,
            target_format,
            target_size: frame.target_size(),
            frame_number: frame.frame_number(),
            draw_count: completion.draw_count,
            triangle_count: completion.triangle_count,
            readback_complete: completion.readback_complete,
        }
    }

    /// Return the explicit adapter policy requested for this image.
    pub const fn requested_policy(&self) -> AdapterPolicy {
        self.requested_policy
    }

    /// Return the exact adapter identity copied from the backend.
    pub const fn adapter(&self) -> &AdapterIdentity {
        &self.adapter
    }

    /// Return the actual target format.
    pub const fn target_format(&self) -> TargetFormat {
        self.target_format
    }

    /// Return the actual target size.
    pub const fn target_size(&self) -> TargetSize {
        self.target_size
    }

    /// Return the submitted lifecycle frame number.
    pub const fn frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Return the submitted draw count.
    pub const fn draw_count(&self) -> usize {
        self.draw_count
    }

    /// Return the submitted triangle count.
    pub const fn triangle_count(&self) -> usize {
        self.triangle_count
    }

    /// Return whether the completed GPU readback was copied into this image.
    pub const fn readback_complete(&self) -> bool {
        self.readback_complete
    }
}

/// A tightly packed, top-left-origin RGBA8 render result.
pub struct RenderedImage {
    size: TargetSize,
    rgba8: Box<[u8]>,
    receipt: BackendReceipt,
}

impl RenderedImage {
    /// Validate exact target dimensions and retain the backend receipt.
    pub fn try_new(
        size: TargetSize,
        rgba8: impl Into<Box<[u8]>>,
        receipt: BackendReceipt,
    ) -> Result<Self, RenderError> {
        let rgba8 = rgba8.into();
        if rgba8.len() != size.rgba8_byte_len() {
            return Err(RenderError::Readback);
        }
        if !receipt.readback_complete() {
            return Err(RenderError::Readback);
        }
        if receipt.target_size() != size {
            return Err(RenderError::ReceiptMismatch);
        }
        Ok(Self {
            size,
            rgba8,
            receipt,
        })
    }

    /// Return the image target size.
    pub const fn size(&self) -> TargetSize {
        self.size
    }

    /// Borrow tightly packed top-left-origin RGBA8 bytes.
    pub fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }

    /// Return backend completion facts.
    pub const fn receipt(&self) -> &BackendReceipt {
        &self.receipt
    }
}
