//! Independent owner libraries. Symbols are versioned; handles never cross owners.
//! Loading and destruction occur outside the render section. Trusted local libraries
//! are executable code; the CLI requires explicit paths and never searches siblings.
use libloading::Library;
use std::{
    ffi::{CString, c_void},
    path::Path,
};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type Process = unsafe extern "C" fn(*mut c_void, *const f64, *mut f64, u32) -> i32;
type Destroy = unsafe extern "C" fn(*mut c_void);
pub struct Dsp {
    handle: *mut c_void,
    process: Process,
    destroy: Destroy,
    reset: Option<unsafe extern "C" fn(*mut c_void)>,
    pub delay: u32,
    outputs: usize,
    max_block: usize,
    _library: Library,
}
impl Dsp {
    /// Loads an explicitly selected trusted owner library. All ABI signatures must
    /// match the owner's v1 contract; callers cannot supply untrusted plugin code.
    pub fn load(path: &Path, owner: &str, block: u32) -> Result<Self> {
        if !matches!(owner, "pa" | "fx") {
            return Err("unknown DSP owner".into());
        }
        // SAFETY: versioned symbols below have the documented matching owner ABI.
        unsafe {
            let library = Library::new(path)?;
            let create = *library.get::<unsafe extern "C" fn(u32, u32) -> *mut c_void>(
                format!("shr_{owner}_v1_create").as_bytes(),
            )?;
            let process = *library.get::<Process>(format!("shr_{owner}_v1_process").as_bytes())?;
            let destroy = *library.get::<Destroy>(format!("shr_{owner}_v1_destroy").as_bytes())?;
            let reset = if owner == "fx" {
                Some(*library.get::<unsafe extern "C" fn(*mut c_void)>(b"shr_fx_v1_reset")?)
            } else {
                None
            };
            let delay_fn =
                if owner == "fx" {
                    Some(*library.get::<unsafe extern "C" fn(*mut c_void) -> u32>(
                        b"shr_fx_v1_delay_frames",
                    )?)
                } else {
                    None
                };
            let handle = create(48000, block);
            if handle.is_null() {
                return Err("DSP create refused configuration".into());
            }
            let delay = delay_fn.map_or(0, |f| f(handle));
            Ok(Self {
                handle,
                process,
                destroy,
                reset,
                delay,
                outputs: if owner == "pa" { 6 } else { 2 },
                max_block: block as usize,
                _library: library,
            })
        }
    }
    pub fn process(&mut self, input: &[f64], output: &mut [f64], outputs: usize) -> bool {
        if input.is_empty()
            || !input.len().is_multiple_of(2)
            || output.len() != input.len() / 2 * outputs
            || outputs != self.outputs
            || input.len() / 2 > self.max_block
        {
            return false;
        }
        // SAFETY: live exclusive handle, complete buffers; owner checks max frames.
        unsafe {
            (self.process)(
                self.handle,
                input.as_ptr(),
                output.as_mut_ptr(),
                (input.len() / 2) as u32,
            ) == 0
        }
    }
    pub fn reset(&mut self) {
        if let Some(reset) = self.reset {
            unsafe {
                reset(self.handle);
            }
        }
    }
}
impl Drop for Dsp {
    fn drop(&mut self) {
        unsafe {
            (self.destroy)(self.handle);
        }
    }
}

type Push = unsafe extern "C" fn(*mut c_void, u64, *const f64, u32) -> i32;
type Finish = unsafe extern "C" fn(*mut c_void) -> i32;
pub struct Recorder {
    handle: *mut c_void,
    push: Push,
    finish: Finish,
    fault: unsafe extern "C" fn(*mut c_void, u32),
    _library: Library,
}
impl Recorder {
    pub fn create(path: &Path, directory: &Path, block: u32, epoch: u64) -> Result<Self> {
        let name = CString::new(directory.as_os_str().as_encoded_bytes())?;
        unsafe {
            let library = Library::new(path)?;
            let create = *library.get::<unsafe extern "C" fn(
                *const std::ffi::c_char,
                u32,
                u32,
                u32,
                u32,
                u64,
            ) -> *mut c_void>(b"shr_rec_v1_create")?;
            let push = *library.get::<Push>(b"shr_rec_v1_push")?;
            let finish = *library.get::<Finish>(b"shr_rec_v1_finish")?;
            let fault =
                *library.get::<unsafe extern "C" fn(*mut c_void, u32)>(b"shr_rec_v1_mark_fault")?;
            let source_bits = *library.get::<unsafe extern "C" fn(*mut c_void, u32) -> i32>(
                b"shr_rec_v1_set_source_bits",
            )?;
            let handle = create(name.as_ptr(), 48000, 8, block, 256, epoch);
            if handle.is_null() {
                return Err("recorder refused new take".into());
            }
            if source_bits(handle, 24) != 0 {
                fault(handle, 3);
                finish(handle);
                return Err("recorder source precision metadata rejected".into());
            }
            Ok(Self {
                handle,
                push,
                finish,
                fault,
                _library: library,
            })
        }
    }
    pub fn push(&mut self, frame: u64, samples: &[f64]) -> i32 {
        if !samples.len().is_multiple_of(8) {
            return -1;
        }
        unsafe {
            (self.push)(
                self.handle,
                frame,
                samples.as_ptr(),
                (samples.len() / 8) as u32,
            )
        }
    }
    pub fn fault(&mut self, code: u32) {
        unsafe {
            (self.fault)(self.handle, code);
        }
    }
    pub fn finish(mut self) -> i32 {
        let result = unsafe { (self.finish)(self.handle) };
        self.handle = std::ptr::null_mut();
        result
    }
}
impl Drop for Recorder {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            self.fault(1);
            unsafe {
                (self.finish)(self.handle);
            }
        }
    }
}
