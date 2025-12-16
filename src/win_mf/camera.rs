use super::mf::*;
use crate::CameraDevice;

use std::{sync::mpsc::*, time::Duration};

use windows::Win32::Media::MediaFoundation::*;

#[allow(unused)]
#[derive(Debug)]
pub struct Camera {
    engine: IMFCaptureEngine,
    device: Device,
    event_rx: Receiver<CaptureEngineEvent>,
    sample_rx: Receiver<Option<IMFSample>>,
    event_cb: IMFCaptureEngineOnEventCallback,
    sample_cb: IMFCaptureEngineOnSampleCallback,
}

#[derive(Debug)]
pub struct Frame {
    buffer: LockedBuffer,
}

pub struct FrameData<'a> {
    data: &'a [u8],
}

impl Camera {
    pub fn new_default_device() -> Option<Self> {
        co_initialize_multithreaded();
        media_foundation_startup().ok()?;

        let engine = new_capture_engine().ok()?;
        let (event_tx, event_rx) = channel::<CaptureEngineEvent>();
        let (sample_tx, sample_rx) = channel::<Option<IMFSample>>();
        let event_cb = CaptureEventCallback { event_tx }.into();
        let sample_cb = CaptureSampleCallback { sample_tx }.into();

        let devices = Device::enum_devices();
        let Some(device) = devices.first().cloned() else { todo!() };

        init_capture_engine(&engine, Some(&device.source), &event_cb).ok()?;

        let camera = Camera { engine, device, event_rx, sample_rx, event_cb, sample_cb };
        camera.wait_for_event(CaptureEngineEvent::Initialized);
        camera.prepare_source_sink();
        Some(camera)
    }

    pub fn start(&self) {
        unsafe { self.engine.StartPreview().unwrap() }
    }

    pub fn stop(&self) {
        capture_engine_stop_preview(&self.engine).unwrap();
    }

    pub fn wait_for_frame(&self) -> Option<Frame> {
        self.sample_rx
            // TODO sometimes running two engines on the same camera breaks frame delivery, so wait not too long
            .recv_timeout(Duration::from_secs(3))
            .ok()
            .flatten()
            .and_then(|sample| {
                let Some(mt) = capture_engine_sink_get_media_type(&self.engine).ok() else {
                    return None;
                };
                let width = mt.frame_width();
                let height = mt.frame_height();
                sample_to_locked_buffer(&sample, width, height).ok()
            })
            .map(|buffer: LockedBuffer| Frame { buffer })
    }

    pub fn device(&self) -> CameraDevice {
        CameraDevice { id: self.device.id().to_string_lossy().to_string(), name: self.device.name() }
    }

    pub fn set_device(&mut self, device: &CameraDevice) -> bool {
        if device.id == self.device.id().to_string_lossy().to_string() {
            return true;
        }
        let find_device = enum_device_sources()
            .into_iter()
            .filter_map(Device::new)
            .find(|d| d.id().to_string_lossy().to_string() == device.id);
        if let Some(new_device) = find_device {
            let engine = new_capture_engine().unwrap();
            let (event_tx, event_rx) = channel::<CaptureEngineEvent>();
            let (sample_tx, sample_rx) = channel::<Option<IMFSample>>();
            let event_cb = CaptureEventCallback { event_tx }.into();
            let sample_cb = CaptureSampleCallback { sample_tx }.into();

            init_capture_engine(&engine, Some(&new_device.source), &event_cb).unwrap();

            *self = Camera { engine, device: new_device, event_rx, sample_rx, event_cb, sample_cb };
            self.wait_for_event(CaptureEngineEvent::Initialized);
            self.prepare_source_sink();
            self.start(); // TODO watch out about playing state
            return true;
        }
        return false;
    }

    pub fn device_list() -> Vec<CameraDevice> {
        enum_device_sources()
            .into_iter()
            .filter_map(Device::new)
            .map(|d| CameraDevice { id: d.id().to_string_lossy().to_string(), name: d.name() })
            .collect()
    }
}

impl Camera {
    fn prepare_source_sink(&self) {
        capture_engine_prepare_sample_callback(&self.engine, &self.sample_cb).unwrap();
    }

    fn wait_for_event(&self, event: CaptureEngineEvent) {
        self.event_rx.iter().find(|e| e == &event);
    }
}

impl Frame {
    pub fn data(&self) -> FrameData {
        FrameData { data: self.buffer.data() }
    }

    pub fn size_u32(&self) -> (u32, u32) {
        (self.buffer.width, self.buffer.height)
    }
}

impl<'a> FrameData<'a> {
    pub fn data_u8(&self) -> &[u8] {
        self.data
    }

    pub fn data_u32(&self) -> &[u32] {
        let (a, data, b) = unsafe { self.data.align_to() };
        debug_assert!(a.is_empty());
        debug_assert!(b.is_empty());
        data
    }
}
