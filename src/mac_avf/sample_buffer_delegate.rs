use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::{Arc, Condvar, Mutex, Once};

use icrate::Foundation::NSObjectProtocol;
use objc2::mutability::InteriorMutable;
use objc2::{rc::Id, runtime::NSObject, *};

use icrate::ns_string;
use objc2::{
    declare::{Ivar, IvarDrop},
    declare_class, extern_methods, msg_send,
    rc::Allocated,
    runtime::{Object, ProtocolObject, Sel},
    sel, ClassType,
};

use super::{CMSampleBufferRef, SampleBuffer};

declare_class!(
    pub struct SampleBufferDelegate {
        slot: IvarDrop<Box<u8>, "_slot">,
    }

    mod ivars;

    unsafe impl ClassType for SampleBufferDelegate {
        type Super = NSObject;
        type Mutability = mutability::InteriorMutable;
        const NAME: &'static str = "SampleBufferDelegate";
    }

    unsafe impl SampleBufferDelegate {
        #[method(captureOutput:didOutputSampleBuffer:fromConnection:)]
        unsafe fn on_output_sample_buffer(
            &self,
            capture_output: *const c_void,
            sample_buffer: CMSampleBufferRef,
            connection: *const c_void,
        ) {
            println!("on_output_sample_buffer")
        }

        #[method(captureOutput:didDropSampleBuffer:fromConnection:)]
        unsafe fn on_drop_sample_buffer(
            &self,
            capture_output: *const c_void,
            sample_buffer: CMSampleBufferRef,
            connection: *const c_void,
        ) {
            println!("on_drop_sample_buffer")
        }
    }

    unsafe impl NSObjectProtocol for SampleBufferDelegate {}
);

impl SampleBufferDelegate {
    pub fn new() -> Id<Self> {
        unsafe { msg_send_id![Self::class(), new] }
    }
}

impl Drop for SampleBufferDelegate {
    fn drop(&mut self) {
        println!("SampleBufferDelegate Drop");
    }
}

#[test]
fn msg_send_to_on_output_sample_buffer() {
    let delegate = SampleBufferDelegate::new();
    let output: *const c_void = null();
    let buffer: CMSampleBufferRef = null_mut();
    let connection: *const c_void = null();
    let () = unsafe {
        msg_send![&delegate, captureOutput: output didOutputSampleBuffer: buffer fromConnection: connection]
    };
}

#[test]
fn msg_send_to_on_drop_sample_buffer() {
    let delegate = SampleBufferDelegate::new();
    let output: *const c_void = null();
    let buffer: CMSampleBufferRef = null_mut();
    let connection: *const c_void = null();
    let () = unsafe {
        msg_send![&delegate, captureOutput: output didDropSampleBuffer: buffer fromConnection: connection]
    };
}

// impl AVCaptureVideoDataOutputSampleBufferDelegate for SampleBufferDelegate {
//     fn on_output_sample_buffer(
//         &mut self,
//         capture_output: *const c_void,
//         sample_buffer: CMSampleBufferRef,
//         connection: *const c_void,
//     ) {
//         let state = self.get_slot_value();
//         self.set_slot_value(State {
//             frame_counter: state.frame_counter + 1,
//         });

//         let sample_buffer = SampleBuffer::new(sample_buffer);
//         println!("{:?}", sample_buffer);
//     }

//     fn on_drop_sample_buffer(&self, capture_output: (), sample_buffer: (), connection: ()) {
//         todo!()
//     }
// }

// // TODO Protocol::protocols

pub type Slot = (Mutex<State>, Condvar);

// impl SampleBufferDelegate {
//     pub fn new() -> Id<Self> {
//         let mut this: Id<Self> = INSObject::new();
//         let slot: Box<Arc<Slot>> = Box::new(Arc::new((
//             Mutex::new(State { frame_counter: 0 }),
//             Condvar::new(),
//         )));
//         this.set_slot(slot);
//         this
//     }

//     fn register_ivars(decl: &mut ClassDecl) {
//         decl.add_ivar::<*mut c_void>("slot");
//     }

//     pub fn clone_slot(&self) -> Arc<Slot> {
//         let ptr = unsafe {
//             let obj = &*(self as *const _ as *const Object);
//             obj.get_ivar::<*mut c_void>("slot")
//         };
//         let slot: Box<Arc<Slot>> = unsafe { Box::from_raw(ptr.cast()) };
//         let clone = *slot.clone();
//         let _ = Box::into_raw(slot);
//         clone
//     }

//     fn set_slot_value(&mut self, value: State) {
//         let ptr = *self.get_mut_slot();
//         let slot: Box<Arc<Slot>> = unsafe { Box::from_raw(ptr.cast()) };
//         *slot.0.lock().unwrap() = value;
//         slot.1.notify_all();
//         let _slot = Box::into_raw(slot);
//     }

//     fn get_slot_value(&self) -> State {
//         let ptr = unsafe {
//             let obj = &*(self as *const _ as *const Object);
//             obj.get_ivar::<*mut c_void>("slot")
//         };
//         let slot: Box<Arc<Slot>> = unsafe { Box::from_raw(ptr.cast()) };
//         let value = slot.0.lock().unwrap().clone();
//         let _slot = Box::into_raw(slot);
//         value
//     }

//     fn set_slot(&mut self, slot: Box<Arc<Slot>>) {
//         self.release_slot();
//         let ptr = Box::into_raw(slot).cast();
//         *self.get_mut_slot() = ptr;
//     }

//     fn get_mut_slot(&mut self) -> &mut *mut c_void {
//         unsafe {
//             let obj = &mut *(self as *mut _ as *mut Object);
//             obj.get_mut_ivar::<*mut c_void>("slot")
//         }
//     }

//     fn release_slot(&mut self) {
//         let ptr = *self.get_mut_slot();
//         if !ptr.is_null() {
//             let _slot: Box<Arc<Slot>> = unsafe { Box::from_raw(ptr.cast()) };
//             *self.get_mut_slot() = null_mut();
//         }
//     }
// }

// static REGISTER_CLASS: Once = Once::new();

// impl INSObject for SampleBufferDelegate {
//     fn class() -> &'static Class {
//         REGISTER_CLASS.call_once(|| {
//             let superclass = NSObject::class();
//             let mut decl = ClassDecl::new("SampleBufferDelegate", superclass).unwrap();

//             Self::register_ivars(&mut decl);

//             unsafe {
//                 decl.add_method(
//                     sel!(captureOutput:didOutputSampleBuffer:fromConnection:),
//                     on_output_sample_buffer as extern "C" fn(&mut Object, Sel, _, _, _),
//                 )
//             };

//             unsafe {
//                 decl.add_method(
//                     sel!(captureOutput:didDropSampleBuffer:fromConnection:),
//                     on_drop_sample_buffer as extern "C" fn(&mut Object, Sel, _, _, _),
//                 )
//             };

//             decl.register();

//             extern "C" fn on_output_sample_buffer(
//                 this: &mut Object,
//                 _cmd: Sel,
//                 capture_output: *const c_void,
//                 sample_buffer: *mut c_void,
//                 connection: *const c_void,
//             ) {
//                 let that: *mut SampleBufferDelegate = (this as *mut Object).cast();
//                 let that = unsafe { that.as_mut().unwrap() };
//                 SampleBufferDelegate::on_output_sample_buffer(
//                     that,
//                     capture_output,
//                     sample_buffer.cast(),
//                     connection,
//                 )
//             }

//             extern "C" fn on_drop_sample_buffer(
//                 this: &mut Object,
//                 _cmd: Sel,
//                 capture_output: *const c_void,
//                 sample_buffer: *const c_void,
//                 connection: *const c_void,
//             ) {
//                 println!("DROP {:?}", this as *const Object);
//             }
//         });

//         Class::get("SampleBufferDelegate").unwrap()
//     }
// }

// #[test]
// fn main() {
//     println!();
//     let mut delegate = SampleBufferDelegate::new();
//     let slot = delegate.clone_slot();
//     if let Ok(v) = slot.0.lock() {
//         println!("{v:?}");
//     }
//     delegate.set_slot_value(State { frame_counter: 2 });
//     if let Ok(v) = slot.0.lock() {
//         println!("{v:?}");
//     }
//     delegate.release_slot();
// }

#[derive(Debug, Clone)]
pub struct State {
    pub frame_counter: usize,
}
