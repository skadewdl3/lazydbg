use std::alloc::{GlobalAlloc, Layout, System};
use std::hint::black_box;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Widget};
use reactatui::prelude::*;

struct CountingAllocator;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static ALLOCATED_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[component]
fn HookView() -> TuiNode<'static> {
    let count = state(|| 0_u64);
    let doubled = memo(|| count.get() * 2);
    black_box(*doubled);
    tui! { <Paragraph::new("reactatui") /> }
}

fn sample(mut render: impl FnMut(), iterations: usize) -> (Duration, usize, usize) {
    render();
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED_BYTES.store(0, Ordering::Relaxed);
    let started = Instant::now();
    for _ in 0..iterations {
        render();
    }
    (
        started.elapsed(),
        ALLOCATIONS.load(Ordering::Relaxed),
        ALLOCATED_BYTES.load(Ordering::Relaxed),
    )
}

fn report(name: &str, result: (Duration, usize, usize), iterations: usize) {
    let (elapsed, allocations, bytes) = result;
    println!(
        "{name:24} {:8.1} ns/frame  {:6.2} allocs/frame  {:8.1} bytes/frame",
        elapsed.as_nanos() as f64 / iterations as f64,
        allocations as f64 / iterations as f64,
        bytes as f64 / iterations as f64,
    );
}

fn main() {
    const ITERATIONS: usize = 100_000;
    let area = Rect::new(0, 0, 80, 24);

    let mut direct_buffer = Buffer::empty(area);
    let direct = sample(
        || Paragraph::new("reactatui").render(area, &mut direct_buffer),
        ITERATIONS,
    );

    let typed_runtime = Runtime::new();
    let mut typed_buffer = Buffer::empty(area);
    let typed = sample(
        || {
            typed_runtime.render_to_buffer(&mut typed_buffer, area, || {
                view(Paragraph::new("reactatui"))
            });
        },
        ITERATIONS,
    );

    let hook_runtime = Runtime::new();
    let mut hook_buffer = Buffer::empty(area);
    let hooks = sample(
        || hook_runtime.render_to_buffer(&mut hook_buffer, area, HookView),
        ITERATIONS,
    );

    println!("{ITERATIONS} steady-state 80x24 renders");
    report("ratatui paragraph", direct, ITERATIONS);
    report("runtime typed view", typed, ITERATIONS);
    report("component + hooks", hooks, ITERATIONS);
}
