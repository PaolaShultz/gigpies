use gigpies::mixer::Mixer;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
thread_local! {static ACTIVE:Cell<bool>=const {Cell::new(false)};static EVENTS:Cell<usize>=const {Cell::new(0)};}
struct Guard;
unsafe impl GlobalAlloc for Guard {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ACTIVE.with(|a| {
            if a.get() {
                EVENTS.with(|n| n.set(n.get() + 1))
            }
        });
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        ACTIVE.with(|a| {
            if a.get() {
                EVENTS.with(|n| n.set(n.get() + 1))
            }
        });
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOC: Guard = Guard;
#[test]
fn public_process_has_no_heap_allocation_or_retirement() {
    let mut m = Mixer::default();
    let prepared = gigpies::mixer::Prepared::edits(&[gigpies::control_model::Edit {
        target: gigpies::control_model::Target::Fader {
            input: "input-01".into(),
        },
        value: gigpies::control_model::Value::Integer(0),
    }])
    .unwrap();
    m.schedule(prepared, 1).unwrap();
    let input = [[0.5; 8]; 1024];
    let mut output = [[0.0; 4]; 1024];
    EVENTS.with(|n| n.set(0));
    ACTIVE.with(|a| a.set(true));
    let result = m.process(&input, &mut output);
    ACTIVE.with(|a| a.set(false));
    assert!(result.is_ok());
    assert_eq!(EVENTS.with(Cell::get), 0);
}

#[test]
fn process_error_and_fault_paths_do_not_allocate_or_deallocate() {
    for kind in 0..3 {
        let mut m = if kind == 1 {
            Mixer::new(u64::MAX)
        } else {
            Mixer::default()
        };
        let input = [[f64::NAN; 8]; 1];
        let mut output = [[0.0; 4]; 1];
        EVENTS.with(|n| n.set(0));
        ACTIVE.with(|a| a.set(true));
        let result = if kind == 0 {
            m.process(&input, &mut [])
        } else {
            m.process(&input, &mut output)
        };
        ACTIVE.with(|a| a.set(false));
        assert_eq!(EVENTS.with(Cell::get), 0);
        assert_eq!(result.is_err(), kind != 2);
    }
}
