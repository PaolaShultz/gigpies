//! Opt-in matched application benchmark. Sink timings exclude terminal presentation.
use serde_json::{Value, json};
use shr_instrument_panel::{
    Material,
    scene::{self, Primitive},
    terminal::ratatui::{
        Terminal, TerminalOptions, Viewport, backend::CrosstermBackend, layout::Rect,
    },
};
use std::{
    cell::Cell,
    fs,
    io::{self, Write},
    rc::Rc,
    time::Instant,
};
#[derive(Clone, Default)]
struct Sink(Rc<Cell<usize>>);
impl Write for Sink {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        self.0.set(self.0.get() + b.len());
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let dir = std::path::Path::new(&args[1]);
    let out = std::path::Path::new(&args[2]);
    let mut results = Vec::new();
    for mode in ["pages", "meters", "spectrum", "unchanged"] {
        let path = dir.join(format!("{mode}-scenes.json"));
        if !path.exists() {
            continue;
        }
        let raw: Vec<Value> = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let scenes: Vec<Vec<Primitive>> = raw
            .iter()
            .map(|s| {
                s["primitives"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| {
                        let n = |k: &str| p[k].as_u64().unwrap_or(0) as u32;
                        let s = |k: &str| p[k].as_str().unwrap_or("").to_string();
                        match p["kind"].as_str().unwrap() {
                            "rect" => Primitive::Rect {
                                x: n("x"),
                                y: n("y"),
                                w: n("w"),
                                h: n("h"),
                                fill: s("color"),
                            },
                            "line" => Primitive::Line {
                                x1: n("x"),
                                y1: n("y"),
                                x2: n("x2"),
                                y2: n("y2"),
                                color: s("color"),
                            },
                            "text" => Primitive::Text {
                                x: n("x"),
                                y: n("y"),
                                value: s("value"),
                                color: s("color"),
                            },
                            _ => panic!("unknown primitive"),
                        }
                    })
                    .collect()
            })
            .collect();
        let sink = Sink::default();
        let count = sink.0.clone();
        let mut t = Terminal::with_options(
            CrosstermBackend::new(sink),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, 160, 45)),
            },
        )
        .unwrap();
        let mut ns = Vec::new();
        let mut bytes = Vec::new();
        for i in 0..10 {
            t.draw(|f| {
                scene::draw(&scenes[i % scenes.len()], f.buffer_mut(), Material::Glyph);
            })
            .unwrap();
        }
        let mut t = Terminal::with_options(
            CrosstermBackend::new(Sink(count.clone())),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::new(0, 0, 160, 45)),
            },
        )
        .unwrap();
        for i in 0..100 {
            let s = &scenes[i % scenes.len()];
            let before = count.get();
            let start = Instant::now();
            t.draw(|f| {
                scene::draw(s, f.buffer_mut(), Material::Glyph);
            })
            .unwrap();
            let time = start.elapsed().as_nanos() as u64;
            ns.push(time);
            bytes.push(count.get() - before);
        }
        results.push(
            json!({"mode":mode,"ns":ns,"bytes":bytes,"excludes":"PTY, emulator, presentation"}),
        );
    }
    fs::write(out, serde_json::to_vec_pretty(&results).unwrap()).unwrap();
}
