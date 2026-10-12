//! Opt-in real-terminal presentation workload. A framebuffer observer ACKs each
//! frame only after seeing its unique marker. This is not a sink benchmark.
use serde_json::Value;
use shr_instrument_panel::{
    Material,
    scene::{self, Primitive},
    terminal::{
        Console,
        ratatui::{
            crossterm::event::{self, Event, KeyCode, KeyEventKind},
            style::Color,
        },
    },
};
use std::{fs, io::Write, time::Duration};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let raw: Vec<Value> = serde_json::from_slice(&fs::read(&args[1])?)?;
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
    let mut console = Console::open()?;
    let mut timing = fs::File::create(&args[2])?;
    fs::write(
        format!("{}.size", args[2]),
        format!("{:?}", console.terminal.size()?),
    )?;
    for (i, s) in scenes.iter().take(100).enumerate() {
        let mut ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: valid writable timespec for this synchronous clock query.
        if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let start = ts.tv_sec as u128 * 1_000_000_000 + ts.tv_nsec as u128;
        console.terminal.draw(|f| {
            scene::draw(s, f.buffer_mut(), Material::Glyph);
            let area = f.area();
            for x in 0..area.width {
                f.buffer_mut()[(x, area.height - 1)].set_char(' ').set_bg(
                    match (usize::from(x) * 8 / usize::from(area.width)).min(7) {
                        0 => Color::Rgb(255, 0, 255),
                        bit if i & (1 << (bit - 1)) != 0 => Color::White,
                        _ => Color::Black,
                    },
                );
            }
        })?;
        writeln!(timing, "{i} {start}")?;
        timing.flush()?;
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if std::time::Instant::now() > deadline {
                return Err("frame observer timeout".into());
            }
            if event::poll(Duration::from_millis(20))?
                && matches!(event::read()?,Event::Key(k) if k.code==KeyCode::Char(' ') && k.kind==KeyEventKind::Press)
            {
                break;
            }
        }
    }
    Ok(())
}
