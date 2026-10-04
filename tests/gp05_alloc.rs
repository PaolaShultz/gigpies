#![cfg(feature = "hardware-host")]
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
#[ignore = "explicit accepted owner libraries required; owner release allocation evidence complements host guard"]
fn bounded_graph_processing_success_and_faults_do_not_allocate() {
    use gigpies::{
        module_graph::{Manifest, ModuleGraph},
        show::Counter,
    };
    let manifest = Manifest::load(std::path::Path::new(
        &std::env::var("GP05_MANIFEST").unwrap(),
    ))
    .unwrap();
    let mut g = ModuleGraph::load(manifest, 9, 0).unwrap();
    let p = std::env::temp_dir().join(format!("gp05-alloc-{}", std::process::id()));
    std::fs::create_dir(&p).unwrap();
    g.start(&p, "take", Counter(1)).unwrap();
    for _ in 0..1000 {
        g.poll_lifecycle().unwrap();
        if g.status("11111111-1111-4111-8111-111111111111", 0)
            .unwrap()
            .recording
            .unwrap()
            .state
            == "recording"
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    for kind in 0..4 {
        let raw = if kind == 1 {
            [[f64::NAN; 8]; 48]
        } else {
            [[0.; 8]; 48]
        };
        let mix = if kind == 3 {
            [[f64::NAN; 4]; 48]
        } else {
            [[0.; 4]; 48]
        };
        let frame = if kind == 2 {
            1000
        } else if kind == 0 {
            0
        } else {
            48
        };
        EVENTS.with(|n| n.set(0));
        ACTIVE.with(|a| a.set(true));
        let result = g.process(9, frame, &raw, &mix);
        ACTIVE.with(|a| a.set(false));
        assert_eq!(EVENTS.with(Cell::get), 0);
        assert_eq!(result.is_ok(), kind == 0);
    }
    drop(g);
    std::fs::remove_dir_all(p).unwrap();
}
