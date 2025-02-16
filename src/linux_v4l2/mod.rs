use ffimage::color::Bgra;

use v4l::context::Node;
use v4l::io::traits::CaptureStream;

use v4l::video::Capture;
use v4l::*;

use std::marker::PhantomData;

use std::sync::RwLock;

use crate::{InnerCamera, CameraDevice};

pub struct Camera {
    device: RwLock<v4l::Device>,
    device_path: String,
    device_name: Option<String>,
    stream: RwLock<Option<v4l::io::mmap::Stream<'static>>>,
}

fn get_next_best_format(device: &Device) -> Format {
    let _rgb = FourCC::new(b"RGB3");
    let mut fmt = device.format().expect("device.format()");
    let size = device
        .enum_framesizes(fmt.fourcc)
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .size
        .to_discrete()
        .into_iter()
        .last()
        .unwrap();
    fmt.width = size.width;
    fmt.height = size.height;
    fmt
}

#[allow(unused)]
fn display_node(node: &Node) {
    println!(
        "Node {{ index: {}, name: {:?}, path: {:?} }}",
        node.index(),
        node.name(),
        node.path()
    );
}

#[allow(unused)]
fn display_device_formats(device: &Device) {
    println!("Device formats:");
    for fmt in device.enum_formats().unwrap() {
        println!("  {:?}", fmt);

        for size in device.enum_framesizes(fmt.fourcc).unwrap() {
            println!("  {:?}", size);
        }
    }
}

fn enum_devices() -> Vec<Node> {
    v4l::context::enum_devices()
        .into_iter()
        .filter_map(|node| Device::with_path(node.path()).ok().map(|device| (node, device)))
        .filter(|(_, device)| device.format().is_ok())
        .map(|(node, _)| node)
        .collect()
}

impl Camera {
    fn from_node(node: &v4l::context::Node) -> Self {
        let device = v4l::Device::with_path(node.path()).unwrap();
        device.set_format(&get_next_best_format(&device)).unwrap();
        Self {
            device: RwLock::new(device),
            device_path: node.path().to_string_lossy().to_string(),
            device_name: node.name(),
            stream: RwLock::new(None),
        }
    }
}

impl InnerCamera for Camera {
    type Frame = Frame;

    fn new_default_device() -> Self {
        let node = enum_devices().into_iter().next().unwrap();
        Self::from_node(&node)
    }

    fn start(&self) {
        if self.stream.read().unwrap().is_none() {
            let device = self.device.write().unwrap();
            let stream =
                v4l::io::mmap::Stream::with_buffers(&device, v4l::buffer::Type::VideoCapture, 4)
                    .expect("Failed to create buffer stream");
            let _ = self.stream.write().unwrap().insert(stream);
        }
    }

    fn stop(&self) {
        let _ = self.stream.write().unwrap().take();
    }

    fn wait_for_frame(&self) -> Option<Frame> {
        let format = self.device.read().unwrap().format().unwrap();
        let size = (format.width, format.height);
        if let Ok((buf, _meta)) = self.stream.write().unwrap().as_mut().unwrap().next() {
            let data = match &format.fourcc.repr {
                b"RGB3" => buf.to_vec(),
                b"YUYV" => yuyv_to_rgb32(buf, size.0, size.1),
                b"MJPG" => mjpg_to_rgb32(buf),
                _ => panic!("invalid buffer pixelformat"),
            };

            Some(Frame { data, size })
        } else {
            None
        }
    }

    fn device(&self) -> CameraDevice {
        CameraDevice { id: self.device_path.clone(), name: self.device_name.as_ref().unwrap_or(&self.device_path).clone() }
    }

    fn set_device(&mut self, device: &CameraDevice) -> bool {
        if device.id == self.device_path {
            return true;
        }
        let find_device = enum_devices()
            .into_iter()
            .find(|d| d.path().to_string_lossy().to_string() == device.id);
        if let Some(new_device) = find_device {
            *self = Self::from_node(&new_device);
            self.start();
            return true;
        }
        self.stop();
        return false;
    }

    fn device_list() -> Vec<CameraDevice> {
        enum_devices()
            .iter()
            .map(|d| {
                let path = d.path().to_string_lossy().to_string();
                CameraDevice { id: path.clone(), name: d.name().unwrap_or(path) }
            })
            .collect()
    }
}

impl std::fmt::Debug for Camera {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Camera").field("device", &self.device_name).finish()
    }
}

pub struct Frame {
    data: Vec<u8>,
    size: (u32, u32),
}

impl Frame {
    pub fn data(&self) -> FrameData {
        FrameData { data: self.data.clone(), _phantom: PhantomData }
    }

    pub fn size_u32(&self) -> (u32, u32) {
        self.size
    }
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Frame").field("data", &self.data.len()).finish()
    }
}

#[derive(Debug)]
pub struct FrameData<'a> {
    data: Vec<u8>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> FrameData<'a> {
    pub fn data_u8(&self) -> &[u8] {
        &self.data
    }

    pub fn data_u32(&self) -> &[u32] {
        unsafe { self.data.align_to().1 }
    }
}

fn yuyv_to_rgb32(buf: &[u8], w: u32, h: u32) -> Vec<u8> {
    use ffimage::color::Rgb;
    use ffimage::packed::{ImageBuffer, ImageView};
    use ffimage::traits::Convert;
    use ffimage_yuv::{yuv::Yuv, yuyv::Yuyv};

    let yuv422 = ImageView::<Yuyv<u8>>::from_buf(buf, w, h).unwrap();
    let mut yuv444 = ImageBuffer::<Yuv<u8>>::new(w, h, 0u8);
    let mut rgb = ImageBuffer::<Rgb<u8>>::new(w, h, 0u8);
    let mut rgba = ImageBuffer::<Bgra<u8>>::new(w, h, 0u8);
    yuv422.convert(&mut yuv444);
    yuv444.convert(&mut rgb);
    rgb.convert(&mut rgba);

    rgba.into_buf()
}

fn mjpg_to_rgb32(buf: &[u8]) -> Vec<u8> {
    use image::ImageFormat;
    use image::io::Reader;
    use std::io::Cursor;

    let mut reader = Reader::new(Cursor::new(buf));
    reader.set_format(ImageFormat::Jpeg);
    // FIXME: make this api fallible
    let im = reader.decode().expect("mjpg decode failed");
    im.to_rgba8().into_raw()
}
