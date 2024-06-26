#[cfg(target_os = "macos")]
use super::mac_avf as backend;

#[cfg(target_os = "windows")]
use super::win_mf as backend;

#[cfg(target_os = "linux")]
use super::linux_v4l2 as backend;

#[derive(Debug)]
pub struct Camera {
    inner: backend::Camera,
}

#[derive(Debug)]
pub struct Frame {
    inner: backend::Frame,
}

pub struct FrameData<'a> {
    inner: backend::FrameData<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraDevice {
    pub id: String,
    pub name: String,
}

impl Camera {
    pub fn new_default_device() -> Self {
        Self { inner: backend::Camera::new_default_device() }
    }

    pub fn start(&self) {
        self.inner.start();
    }

    pub fn stop(&self) {
        self.inner.stop();
    }

    pub fn wait_for_frame(&self) -> Option<Frame> {
        self.inner.wait_for_frame().map(|inner| Frame { inner })
    }

    pub fn device(&self) -> CameraDevice {
        self.inner.device()
    }

    pub fn set_device(&mut self, device: &CameraDevice) -> bool {
        self.inner.set_device(device)
    }

    pub fn device_list() -> Vec<CameraDevice> {
        backend::Camera::device_list()
    }
}

impl Frame {
    pub fn data(&self) -> FrameData {
        FrameData { inner: self.inner.data() }
    }

    pub fn size_u32(&self) -> (u32, u32) {
        self.inner.size_u32()
    }
}

impl<'a> FrameData<'a> {
    pub fn data_u8(&self) -> &[u8] {
        self.inner.data_u8()
    }

    pub fn data_u32(&self) -> &[u32] {
        self.inner.data_u32()
    }
}

pub(crate) trait InnerCamera: std::fmt::Debug {
    type Frame;

    fn new_default_device() -> Self;
    fn start(&self);
    fn stop(&self);
    fn wait_for_frame(&self) -> Option<Self::Frame>;
    fn device(&self) -> CameraDevice;
    fn set_device(&mut self, device: &CameraDevice) -> bool;
    fn device_list() -> Vec<CameraDevice>;
}
