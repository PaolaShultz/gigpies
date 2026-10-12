//! Opt-in real-terminal key-edge probe. No providers or device commands.
use shr_instrument_panel::terminal::{
    Console,
    ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers},
};
use std::{fs::File, io::Write, time::Duration};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut console = Console::open()?;
    let mut log = File::create(std::env::args().nth(1).ok_or("log path required")?)?;
    writeln!(log, "release_events={}", console.release_events)?;
    console.terminal.draw(|frame| {
        frame.buffer_mut().set_string(
            0,
            0,
            "Input qualification: T, Enter; Ctrl+C exits. NO PROVIDER.",
            shr_instrument_panel::terminal::ratatui::style::Style::default(),
        );
    })?;
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if event::poll(Duration::from_millis(50))? {
            let event = event::read()?;
            writeln!(log, "{event:?}")?;
            log.flush()?;
            if matches!(event,Event::Key(k) if k.code==KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL))
            {
                break;
            }
        }
    }
    Ok(())
}
