//! Configurable SHR PA owner ABI. No speaker DSP is implemented in this host.
//! Prepare and retire on the controller; process and commit have exclusive ownership.
use super::adapters::Result;
use libloading::Library;
use std::{ffi::c_void, path::Path, sync::Arc};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Status {
    pub version: u32,
    pub size: u32,
    pub sample_rate: u32,
    pub max_block: u32,
    pub input_channels: u32,
    pub output_channels: u32,
    pub muted: u32,
    pub quiesced: u32,
    pub fault_latched: u32,
    pub committed: u32,
    pub reserved0: u32,
    pub reserved1: u32,
    pub epoch: u64,
    pub next_frame: u64,
    pub generation: u64,
    pub applied_frame: u64,
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub version: u32,
    pub size: u32,
    pub min_rate: u32,
    pub max_rate: u32,
    pub min_block: u32,
    pub max_block: u32,
    pub max_ports: u32,
    pub max_nodes: u32,
    pub max_operations: u32,
    pub max_delay_samples: u32,
    pub max_json_bytes: u32,
    pub sample_format: u32,
    pub measurement_available: u32,
    pub true_peak_available: u32,
    pub acoustic_protection_available: u32,
    pub physical_io_owned: u32,
}
const _: () = assert!(std::mem::size_of::<Status>() == 80);
const _: () = assert!(std::mem::size_of::<Capabilities>() == 64);
type Handle = *mut c_void;
type Prepare = unsafe extern "C" fn(*const u8, u32, u32) -> Handle;
type Validate = unsafe extern "C" fn(*const u8, u32, *mut u8, u32) -> i32;
type Apply = unsafe extern "C" fn(*mut Handle, Handle, *mut Handle, u64, u64) -> i32;
type Process = unsafe extern "C" fn(Handle, *const f64, *mut f64, u32, u32, u32, u64, u64) -> i32;
type Query = unsafe extern "C" fn(Handle, *mut Status, u32, u32) -> i32;
type Destroy = unsafe extern "C" fn(Handle);
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EqStatus {
    pub version: u32,
    pub size: u32,
    pub eligible: u32,
    pub retirement_occupied: u32,
    pub graph_generation: u64,
    pub eq_generation: u64,
    pub epoch: u64,
    pub next_frame: u64,
    pub remaining: u64,
    pub instance: u64,
}
#[repr(C)]
#[derive(Default)]
struct EqCapabilities {
    version: u32,
    size: u32,
    status_size: u32,
    max_json_bytes: u32,
    sections_per_input: u32,
    selected_inputs: u32,
    extra_work_units: u32,
    reserved: u32,
}
struct EqApi {
    status: unsafe extern "C" fn(Handle, *mut EqStatus, u32, u32) -> i32,
    prepare: unsafe extern "C" fn(Handle, *const u8, u32, u32, u64, u64, u64, u64) -> Handle,
    apply: unsafe extern "C" fn(Handle, *mut Handle, u64, u64) -> i32,
    retire: unsafe extern "C" fn(Handle, *mut Handle) -> i32,
    destroy: Destroy,
    readback: unsafe extern "C" fn(Handle, u32, u32, *mut u8, u32) -> i32,
}
impl EqApi {
    unsafe fn load(l: &Library) -> Option<Self> {
        // Missing, partial and incompatible extensions all retain legacy loading.
        unsafe {
            let caps = *l
                .get::<unsafe extern "C" fn(*mut EqCapabilities, u32, u32) -> i32>(
                    b"shr_pa_eq_v1_capabilities",
                )
                .ok()?;
            let mut c = EqCapabilities::default();
            if caps(&mut c, 1, 32) != 0
                || c.version != 1
                || c.size != 32
                || c.status_size != 64
                || c.max_json_bytes != 32768
                || c.sections_per_input != 39
                || c.selected_inputs != 2
                || c.extra_work_units != 80
                || c.reserved != 0
            {
                return None;
            }
            Some(Self {
                status: *l.get(b"shr_pa_eq_v1_status").ok()?,
                prepare: *l.get(b"shr_pa_eq_v1_prepare").ok()?,
                apply: *l.get(b"shr_pa_eq_v1_apply").ok()?,
                retire: *l.get(b"shr_pa_eq_v1_retire").ok()?,
                destroy: *l.get(b"shr_pa_eq_v1_destroy").ok()?,
                readback: *l.get(b"shr_pa_eq_v1_readback").ok()?,
            })
        }
    }
}
pub struct PreparedEq {
    handle: Handle,
    api: Arc<Api>,
}
impl Drop for PreparedEq {
    fn drop(&mut self) {
        if let Some(eq) = &self.api.eq {
            unsafe {
                (eq.destroy)(self.handle);
            }
        }
    }
}
struct Api {
    eq: Option<EqApi>,
    prepare: Prepare,
    validate: Validate,
    apply: Apply,
    process: Process,
    status: Query,
    destroy: Destroy,
    mute: unsafe extern "C" fn(Handle) -> i32,
    rearm: unsafe extern "C" fn(Handle, u64, u64) -> i32,
    capabilities: Capabilities,
    _library: Library,
}
/// A prepared owner allocation. Drop only outside rendering, including rejected edits.
pub struct Prepared {
    handle: Handle,
    api: Arc<Api>,
}
impl Prepared {
    /// Exclusive read of the fresh owner shape, before host buffer admission.
    pub fn status(&self) -> std::result::Result<Status, i32> {
        if self.handle.is_null() {
            return Err(-1);
        }
        let mut status = Status::default();
        let result = unsafe { (self.api.status)(self.handle, &mut status, 2, 80) };
        if result == 0 { Ok(status) } else { Err(result) }
    }
}
impl Drop for Prepared {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns the pointer or it was consumed and is null.
        unsafe { (self.api.destroy)(self.handle) };
    }
}
/// One active handle and one reserved retired slot. A full slot rejects the next
/// commit until the controller calls retire(); no render-side destruction occurs.
pub struct Pa {
    active: Handle,
    retired: Handle,
    api: Arc<Api>,
    inputs: usize,
    outputs: usize,
    max_block: usize,
}
impl Pa {
    /// Load an explicitly hash-verified trusted owner library off the render path.
    pub fn load(path: &Path, json: &[u8], epoch: u64, frame: u64) -> Result<Self> {
        let mut pa = Self::open(path)?;
        let mut prepared = pa.prepare(json)?;
        let result = pa.commit(&mut prepared, epoch, frame);
        if result != 0 {
            return Err(format!("SHR PA v2 initial commit refused: {result}").into());
        }
        Ok(pa)
    }
    /// Load the ABI without an active graph. Initial prepared ownership is
    /// committed later at the actual render boundary, not preparation time.
    pub fn open(path: &Path) -> Result<Self> {
        // SAFETY: caller selects trusted library; all names/layouts match SHR PA v2.
        let api = unsafe {
            let library = Library::new(path)?;
            let mut capabilities = Capabilities::default();
            let query = *library.get::<unsafe extern "C" fn(*mut Capabilities, u32, u32) -> i32>(
                b"shr_pa_v2_capabilities",
            )?;
            if query(&mut capabilities, 2, 64) != 0
                || capabilities.version != 2
                || capabilities.size != 64
                || capabilities.sample_format != 1
            {
                return Err("SHR PA v2 capabilities mismatch".into());
            }
            Arc::new(Api {
                eq: EqApi::load(&library),
                prepare: *library.get::<Prepare>(b"shr_pa_v2_prepare")?,
                validate: *library.get::<Validate>(b"shr_pa_v2_validate")?,
                apply: *library.get::<Apply>(b"shr_pa_v2_apply")?,
                process: *library.get::<Process>(b"shr_pa_v2_process")?,
                status: *library.get::<Query>(b"shr_pa_v2_status")?,
                destroy: *library.get::<Destroy>(b"shr_pa_v2_destroy")?,
                mute: *library.get(b"shr_pa_v2_mute")?,
                rearm: *library.get(b"shr_pa_v2_rearm")?,
                capabilities,
                _library: library,
            })
        };
        Ok(Self {
            active: std::ptr::null_mut(),
            retired: std::ptr::null_mut(),
            api,
            inputs: 0,
            outputs: 0,
            max_block: 0,
        })
    }
    pub fn eq_status(&self) -> std::result::Result<EqStatus, i32> {
        let eq = self.api.eq.as_ref().ok_or(-5)?;
        let mut s = EqStatus::default();
        let code = unsafe { (eq.status)(self.active, &mut s, 1, 64) };
        if code != 0 {
            return Err(code);
        }
        if s.version != 1 || s.size != 64 || s.eligible > 1 || s.retirement_occupied > 1 {
            return Err(-5);
        }
        Ok(s)
    }
    pub fn prepare_eq(&self, json: &[u8], expected: EqStatus, frame: u64) -> Result<PreparedEq> {
        if json.is_empty() || json.len() > 32768 {
            return Err("EQ JSON bound".into());
        }
        let eq = self.api.eq.as_ref().ok_or("live EQ unavailable")?;
        let current = self.eq_status().map_err(|e| format!("EQ status {e}"))?;
        if current.instance != expected.instance
            || current.graph_generation != expected.graph_generation
            || current.eq_generation != expected.eq_generation
            || current.epoch != expected.epoch
        {
            return Err("stale EQ identity".into());
        }
        let handle = unsafe {
            (eq.prepare)(
                self.active,
                json.as_ptr(),
                json.len() as u32,
                1,
                expected.graph_generation,
                expected.eq_generation,
                expected.epoch,
                frame,
            )
        };
        if handle.is_null() {
            return Err("EQ owner preparation refused".into());
        }
        Ok(PreparedEq {
            handle,
            api: Arc::clone(&self.api),
        })
    }
    /// Narrow boundary operation; preserves buffers and PA hold/arm state.
    pub fn commit_eq(&mut self, p: &mut PreparedEq, epoch: u64, frame: u64) -> i32 {
        if !Arc::ptr_eq(&self.api, &p.api) || p.handle.is_null() {
            return -1;
        }
        let Some(eq) = &self.api.eq else {
            return -5;
        };
        unsafe { (eq.apply)(self.active, &mut p.handle, epoch, frame) }
    }
    /// Controller-only explicit disposal after endpoint/fault.
    pub fn retire_eq(&mut self) {
        if let Some(eq) = &self.api.eq {
            let mut raw = std::ptr::null_mut();
            if unsafe { (eq.retire)(self.active, &mut raw) } == 0 {
                unsafe {
                    (eq.destroy)(raw);
                }
            }
        }
    }
    pub fn eq_readback(&self, indices: [usize; 2]) -> Result<String> {
        let eq = self.api.eq.as_ref().ok_or("live EQ unavailable")?;
        let mut bytes = vec![0; 65536];
        let code = unsafe {
            (eq.readback)(
                self.active,
                indices[0] as u32,
                indices[1] as u32,
                bytes.as_mut_ptr(),
                bytes.len() as u32,
            )
        };
        if code <= 0 {
            return Err(format!("EQ readback {code}").into());
        }
        bytes.truncate(code as usize);
        String::from_utf8(bytes).map_err(Into::into)
    }
    pub fn capabilities(&self) -> Capabilities {
        self.api.capabilities
    }
    pub fn input_channels(&self) -> usize {
        self.inputs
    }
    pub fn output_channels(&self) -> usize {
        self.outputs
    }
    /// Allocation, parsing and coefficient design occur only here in the owner.
    pub fn prepare(&self, json: &[u8]) -> Result<Prepared> {
        if json.is_empty() || json.len() > self.api.capabilities.max_json_bytes as usize {
            return Err("SHR PA v2 JSON resource bound".into());
        }
        let mut reason = [0u8; 512];
        // SAFETY: disjoint live input/error storage; owner copies configuration.
        unsafe {
            let result = (self.api.validate)(
                json.as_ptr(),
                json.len() as u32,
                reason.as_mut_ptr(),
                reason.len() as u32,
            );
            if result != 0 {
                let len = reason.iter().position(|v| *v == 0).unwrap_or(reason.len());
                return Err(format!(
                    "SHR PA v2 preparation refused: {}",
                    String::from_utf8_lossy(&reason[..len])
                )
                .into());
            }
            let handle = (self.api.prepare)(json.as_ptr(), json.len() as u32, 2);
            if handle.is_null() {
                return Err("SHR PA v2 preparation failed".into());
            }
            Ok(Prepared {
                handle,
                api: Arc::clone(&self.api),
            })
        }
    }
    /// Boundary-only ownership swap. On refusal prepared and active remain owned.
    /// No allocation, deallocation or refcount change; retire() is controller-only.
    pub fn commit(&mut self, prepared: &mut Prepared, epoch: u64, frame: u64) -> i32 {
        if prepared.handle.is_null() || !Arc::ptr_eq(&self.api, &prepared.api) {
            return -1;
        }
        if !self.retired.is_null() {
            return -3;
        }
        // Query the fresh state before swapping, so shape is known without fallible
        // work after ownership has transferred.
        let mut status = Status::default();
        unsafe {
            let result = (self.api.status)(prepared.handle, &mut status, 2, 80);
            if result != 0 {
                return result;
            }
            let result = (self.api.apply)(
                &mut self.active,
                prepared.handle,
                &mut self.retired,
                epoch,
                frame,
            );
            if result == 0 {
                prepared.handle = std::ptr::null_mut();
                self.inputs = status.input_channels as usize;
                self.outputs = status.output_channels as usize;
                self.max_block = status.max_block as usize;
            }
            result
        }
    }
    /// Controller only, after exclusive render use has returned.
    pub fn retire(&mut self) {
        unsafe { (self.api.destroy)(self.retired) };
        self.retired = std::ptr::null_mut();
    }
    pub fn status(&self) -> std::result::Result<Status, i32> {
        let mut status = Status::default();
        let result = unsafe { (self.api.status)(self.active, &mut status, 2, 80) };
        if result == 0 { Ok(status) } else { Err(result) }
    }
    pub fn mute(&mut self) -> i32 {
        unsafe { (self.api.mute)(self.active) }
    }
    pub fn rearm(&mut self, epoch: u64, frame: u64) -> i32 {
        unsafe { (self.api.rearm)(self.active, epoch, frame) }
    }
    pub fn process(&mut self, input: &[f64], output: &mut [f64], epoch: u64, frame: u64) -> i32 {
        if self.inputs == 0 || input.is_empty() || !input.len().is_multiple_of(self.inputs) {
            return -1;
        }
        let frames = input.len() / self.inputs;
        if frames > self.max_block || frames.checked_mul(self.outputs) != Some(output.len()) {
            return -1;
        }
        // SAFETY: exclusive active state, prepared exact shapes and disjoint Rust slices.
        unsafe {
            (self.api.process)(
                self.active,
                input.as_ptr(),
                output.as_mut_ptr(),
                self.inputs as u32,
                self.outputs as u32,
                frames as u32,
                epoch,
                frame,
            )
        }
    }
}
impl Drop for Pa {
    fn drop(&mut self) {
        self.retire();
        unsafe { (self.api.destroy)(self.active) };
    }
}
