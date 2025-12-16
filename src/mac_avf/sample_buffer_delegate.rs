use std::ffi::c_void;
use std::sync::{Arc, Condvar, Mutex};
use std::sync::atomic::AtomicPtr;

use objc2_av_foundation::AVCaptureVideoDataOutputSampleBufferDelegate;
use objc2_core_media::CMSampleBuffer;
use objc2::{AnyThread, DefinedClass, define_class, msg_send};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol};

pub struct SampleBufferIvars {
    slot: Box<Arc<Slot>>,
}

define_class!(
    #[unsafe(super(NSObject))]

    #[ivars = SampleBufferIvars]
    pub struct SampleBufferDelegate;

    unsafe impl NSObjectProtocol for SampleBufferDelegate {}
    unsafe impl AVCaptureVideoDataOutputSampleBufferDelegate for SampleBufferDelegate {}

    impl SampleBufferDelegate {
        #[unsafe(method(captureOutput:didOutputSampleBuffer:fromConnection:))]
        fn on_output_sample_buffer(
            &self,
            _capture_output: *const c_void,
            sample_buffer: *mut CMSampleBuffer,
            _connection: *const c_void,
        ) {
            self.set_slot(unsafe { Retained::retain(sample_buffer) });
        }

        #[unsafe(method(captureOutput:didDropSampleBuffer:fromConnection:))]
        unsafe fn on_drop_sample_buffer(
            &self,
            _capture_output: *const c_void,
            _sample_buffer: *mut CMSampleBuffer,
            _connection: *const c_void,
        ) {
            // NOTE: this will wake up wake_for_sample with a None sample
            // when do we actually want to do that?
            self.set_slot(None);
        }
    }
);

use std::fmt;
impl fmt::Debug for SampleBufferDelegate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SampleBufferDelegate")
            .field("address", &format_args!("{:p}", self))
         .finish()
    }
}

impl SampleBufferDelegate {
    pub fn new() -> Retained<Self> {
        let this = SampleBufferDelegate::alloc();
        let this = this.set_ivars(SampleBufferIvars {
            slot: Box::new(Arc::new(Slot::new())),
        });
        let this = unsafe { msg_send![super(this), init] };
        this
    }

    pub fn slot(&self) -> Arc<Slot> {
        (*self.ivars().slot).clone()
    }

    fn set_slot(&self, sample: Option<Retained<CMSampleBuffer>>) {
        let slot = &self.ivars().slot;
        slot.set_sample(sample);
        slot.notify_all();
    }
}

#[derive(Debug)]
pub struct Slot {
    sample: AtomicPtr<CMSampleBuffer>,
    state: Mutex<State>,
    condvar: Condvar,
}

impl Slot {
    fn new() -> Self {
        Self {
            sample: AtomicPtr::new(std::ptr::null_mut()),
            state: Mutex::new(State { frame_counter: 0 }),
            condvar: Condvar::new(),
        }
    }

    pub fn wait_for_sample(&self) -> Option<Retained<CMSampleBuffer>> {
        let mut _guard = self.state.lock().unwrap();
        _guard = self.condvar.wait(_guard).unwrap();
        let ptr = self.sample.swap(std::ptr::null_mut(), std::sync::atomic::Ordering::Relaxed);
        unsafe { Retained::from_raw(ptr) }
    }

    fn set_sample(&self, sample: Option<Retained<CMSampleBuffer>>) {
        let sample_ptr = sample.map(|x| Retained::into_raw(x)).unwrap_or(std::ptr::null_mut());
        let old_sample = self.sample.swap(sample_ptr, std::sync::atomic::Ordering::Relaxed);
        // drop old sample
        unsafe { Retained::from_raw(old_sample); }
    }

    fn notify_all(&self) {
        self.condvar.notify_all();
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        let old_sample = self.sample.swap(std::ptr::null_mut(), std::sync::atomic::Ordering::Relaxed);
        unsafe { Retained::from_raw(old_sample); }
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub frame_counter: usize,
}

#[test]
fn msg_send_to_on_output_sample_buffer() {
    use std::ptr::null;
    let delegate = SampleBufferDelegate::new();
    let output: *const c_void = null();
    let buffer: *mut CMSampleBuffer = std::ptr::null_mut();
    let connection: *const c_void = null();
    let () = unsafe {
        msg_send![&delegate, captureOutput: output didOutputSampleBuffer: buffer fromConnection: connection]
    };
}

#[test]
fn msg_send_to_on_drop_sample_buffer() {
    use std::ptr::null;
    let delegate = SampleBufferDelegate::new();
    let output: *const c_void = null();
    let buffer: *mut CMSampleBuffer = std::ptr::null_mut();
    let connection: *const c_void = null();
    let () = unsafe {
        msg_send![&delegate, captureOutput: output didDropSampleBuffer: buffer fromConnection: connection]
    };
}

#[test]
fn slot() {
    let delegate = SampleBufferDelegate::new();
    println!("slot {:?}", delegate.slot());
}
