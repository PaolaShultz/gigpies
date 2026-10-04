use gigpies::analysis_stream::{Descriptor, Tap, synthetic_inputs};
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
fn raw_tap_full_queue_and_errors_do_not_allocate_or_retire() {
    let (mut tap, _q) = Tap::new(&Descriptor::new(9, 0)).unwrap();
    let raw = synthetic_inputs(0);
    EVENTS.with(|n| n.set(0));
    ACTIVE.with(|a| a.set(true));
    for i in 0..100 {
        assert!(tap.offer(i * 48, &raw, 0).is_ok());
    }
    let error = tap.offer(0, &raw, 0);
    ACTIVE.with(|a| a.set(false));
    assert!(error.is_err());
    assert_eq!(tap.dropped_windows, 8);
    assert_eq!(EVENTS.with(Cell::get), 0);
}
