use std::sync::Arc;

use dispatch2::{DispatchQueue, DispatchRetained};
use objc2_foundation::{
    NSDictionary,
    NSNumber,
    NSString,
};
use objc2_av_foundation::{
    AVCaptureDevice,
    AVCaptureDeviceInput,
    AVCaptureVideoDataOutput,
    AVCaptureSession,
};
use objc2_core_foundation::CFRetained;
use objc2_core_media::CMSampleBuffer;
use objc2_core_video::{
    kCVPixelBufferPixelFormatTypeKey,
    kCVPixelFormatType_32BGRA,
    CVPixelBufferLockFlags,
    CVImageBuffer,
    CVPixelBufferLockBaseAddress,
    CVPixelBufferGetBaseAddress,
    CVPixelBufferGetBytesPerRow,
    CVPixelBufferGetWidth,
    CVPixelBufferGetHeight,
    CVPixelBufferIsPlanar,
    CVPixelBufferGetPlaneCount,
    CVPixelBufferGetDataSize,
    CVPixelBufferGetPixelFormatType,
    CVPixelBufferGetBaseAddressOfPlane,
    CVPixelBufferGetBytesPerRowOfPlane,
    CVPixelBufferGetHeightOfPlane,
    CVPixelBufferUnlockBaseAddress,
};
use objc2::runtime::ProtocolObject;
use objc2::rc::Retained;

use crate::CameraDevice;
use crate::mac_avf::Slot;
use crate::mac_avf::SampleBufferDelegate;

#[derive(Debug)]
pub struct Camera {
    device: Retained<AVCaptureDevice>,
    input: Retained<AVCaptureDeviceInput>,
    #[allow(unused)]
    output: Retained<AVCaptureVideoDataOutput>,
    session: Retained<AVCaptureSession>,
    slot: Arc<Slot>,
    _queue: DispatchRetained<DispatchQueue>,
}

#[derive(Debug)]
pub struct Frame {
    sample: Retained<CMSampleBuffer>,
}

pub struct FrameData<'a> {
    pixels: Pixels<'a>,
}

impl Camera {
    pub fn new_default_device() -> Option<Self> {
        unsafe {
            let device = AVCaptureDevice::defaultDeviceWithMediaType(objc2_av_foundation::AVMediaTypeVideo?)?;
            let input = AVCaptureDeviceInput::deviceInputWithDevice_error(&device).ok()?;
            let output = AVCaptureVideoDataOutput::new();
            output.setVideoSettings(Some(&*NSDictionary::<NSString>::from_slices(
                &[&*(kCVPixelBufferPixelFormatTypeKey as *const objc2_core_foundation::CFString as *const NSString)],
                &[&*NSNumber::new_u32(kCVPixelFormatType_32BGRA)],
            )));
            let delegate = SampleBufferDelegate::new();
            let slot = delegate.slot();
            let session = AVCaptureSession::new();
            let queue = DispatchQueue::new("kamera-rs", None);
            output.setSampleBufferDelegate_queue(Some(ProtocolObject::from_ref(&*delegate)), Some(&queue));
            std::mem::forget(delegate);

            session.addInput(&input);
            session.addOutput(&output);

            Some(Camera { device, input, output, session, slot, _queue: queue })
        }
    }

    pub fn start(&self) {
        unsafe { self.session.startRunning(); }
    }

    pub fn stop(&self) {
        unsafe { self.session.stopRunning(); }
    }

    pub fn wait_for_frame(&self) -> Option<Frame> {
        self.slot.wait_for_sample().map(|sample| Frame { sample })
    }

    pub fn device(&self) -> CameraDevice {
        unsafe {
            return CameraDevice { id: self.device.uniqueID().to_string(), name: self.device.localizedName().to_string() }
        }
    }

    pub fn set_device(&mut self, device: &CameraDevice) -> bool {
        if device.id == unsafe { self.device.uniqueID().to_string() } {
            return true;
        }
        let find_device = unsafe { AVCaptureDevice::devicesWithMediaType(objc2_av_foundation::AVMediaTypeVideo.unwrap()) }
            .into_iter()
            .find(|d| unsafe { d.uniqueID().to_string() == device.id });
        if let Some(new_device) = find_device {
            let new_input = match unsafe { AVCaptureDeviceInput::deviceInputWithDevice_error(&new_device) } {
                Ok(value) => value,
                Err(_) => return false,
            };
            unsafe { self.session.removeInput(&self.input) };
            self.device = new_device;
            self.input = new_input;
            unsafe { self.session.addInput(&self.input); }
            return true;
        }
        return false;
    }

    pub fn device_list() -> Vec<CameraDevice> {
        unsafe {
            AVCaptureDevice::devicesWithMediaType(objc2_av_foundation::AVMediaTypeVideo.unwrap())
                .iter()
                .map(|device| CameraDevice { id: device.uniqueID().to_string(), name: device.localizedName().to_string() })
                .collect()
        }
    }
}

/// Holds the locked pixel data of a frame and unlocks upon drop.
pub struct Pixels<'a> {
    pub ibuf: CFRetained<CVImageBuffer>,
    pub data: &'a [u8],
    pub u32: &'a [u32],
    pub width: usize,
    pub height: usize,
}

impl<'a> Pixels<'a> {
    fn new(sample: &Retained<CMSampleBuffer>) -> Self {
        // FIXME: no unwrap?
        let ibuf = unsafe { sample.image_buffer().unwrap() };

        assert_eq!(0, unsafe { CVPixelBufferLockBaseAddress(&ibuf, CVPixelBufferLockFlags::ReadOnly) });
        let _address = CVPixelBufferGetBaseAddress(&ibuf);
        let stride = CVPixelBufferGetBytesPerRow(&ibuf);
        let width = CVPixelBufferGetWidth(&ibuf);
        let height = CVPixelBufferGetHeight(&ibuf);
        let is_planar = CVPixelBufferIsPlanar(&ibuf);
        let plane_count = CVPixelBufferGetPlaneCount(&ibuf);
        let _data_size = CVPixelBufferGetDataSize(&ibuf);
        let _fourcc = CVPixelBufferGetPixelFormatType(&ibuf);
        let plane_address = CVPixelBufferGetBaseAddressOfPlane(&ibuf, 0);
        let mut plane_sizes = 0;

        // println!("pixels {:?}", (_address, stride, width, height, is_planar, plane_count, _data_size, fourcc_to_string(_fourcc)));
        if is_planar {
            for index in 0..plane_count {
                let _plane_address = CVPixelBufferGetBaseAddressOfPlane(&ibuf, index);
                let plane_stride = CVPixelBufferGetBytesPerRowOfPlane(&ibuf, index);
                let plane_height = CVPixelBufferGetHeightOfPlane(&ibuf, index);
                // println!("        {:?}", (plane_address, plane_stride, plane_height));
                plane_sizes += plane_stride * plane_height;
            }
        } else {
            plane_sizes += stride * height;
        }

        let data = unsafe { std::slice::from_raw_parts(plane_address as *mut u8, plane_sizes) };
        let (a, u32, b) = unsafe { data.align_to() };
        debug_assert!(a.is_empty() && b.is_empty());
        Self { ibuf, data, u32, width, height }
    }
}

impl Drop for Pixels<'_> {
    fn drop(&mut self) {
        assert_eq!(0, unsafe { CVPixelBufferUnlockBaseAddress(&self.ibuf, CVPixelBufferLockFlags::ReadOnly) });
    }
}

impl Frame {
    pub fn data(&self) -> FrameData {
        FrameData { pixels: Pixels::new(&self.sample) }
    }

    pub fn size_u32(&self) -> (u32, u32) {
        // FIXME: no unwrap?
        let ibuf = unsafe { self.sample.image_buffer().unwrap() };
        let width = CVPixelBufferGetWidth(&ibuf);
        let height = CVPixelBufferGetHeight(&ibuf);
        (width as _, height as _)
    }
}

impl<'a> FrameData<'a> {
    pub fn data_u8(&self) -> &[u8] {
        self.pixels.data
    }

    pub fn data_u32(&self) -> &[u32] {
        self.pixels.u32
    }
}

#[cfg(test)]
const TEST_FRAMES: usize = 3;

#[test]
fn change_device() {
    let mut camera = Camera::new_default_device().unwrap();
    camera.start();

    println!("first camera");
    std::iter::from_fn(|| camera.wait_for_frame())
        .map(|s| println!("{s:?}"))
        .take(TEST_FRAMES)
        .count();

    for device in Camera::device_list() {
        println!("change device to {:?}", device);
        camera.set_device(&device);

        std::iter::from_fn(|| camera.wait_for_frame())
            .map(|s| println!("{s:?}"))
            .take(TEST_FRAMES)
            .count();
        break;
    }
}
