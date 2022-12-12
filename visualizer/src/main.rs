use crossterm::event::{Event, KeyCode, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{event, execute};
use std::io::{self, BufRead};
use std::process::exit;
use std::thread;
use std::time::Duration;
use tui::backend::CrosstermBackend;
use tui::Terminal;

use crate::state::State;
use crate::terminal::Redraw;

#[macro_use]
mod defer;

mod config;
mod err;
mod parse;
mod state;
mod terminal;

fn main() -> Result<(), err::DebugFromDisplay<io::Error>> {
    if atty::is(atty::Stream::Stdin) {
        eprintln!("Input must be piped into stdin.");
        eprintln!();
        eprintln!("All available commands:");
        eprintln!(".vz cn <chart name>");
        eprintln!(".vz sn <series name>");
        eprintln!(".vz xn <x-axis name>");
        eprintln!(".vz yn <y-axis name>");
        eprintln!(".vz xs [<x coord>, ...]");
        eprintln!(".vz ys [<y coord>, ...]");
        return Ok(());
    }

    let stdout = io::stdout();
    let mut stdin = io::stdin().lock();

    let mut terminal = Terminal::new(CrosstermBackend::new(&stdout))?;

    thread::scope(|s| {
        // spawn a background thread to handle user input (for exiting)
        s.spawn(|| {
            let status = {
                defer! { let _ = disable_raw_mode(); }
                defer! { let _ = execute!(&stdout, LeaveAlternateScreen); }

                fn run() -> Result<(), io::Error> {
                    loop {
                        if event::poll(Duration::from_secs(365 * 86400))? {
                            if let Event::Key(key) = event::read()? {
                                let ctrl_c = key.modifiers == KeyModifiers::CONTROL
                                    && key.code == KeyCode::Char('c');
                                let q = key.modifiers == KeyModifiers::NONE
                                    && key.code == KeyCode::Char('q');
                                if ctrl_c || q {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }

                match run() {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("Failed to run input thread: {:?}", e);
                        1337
                    }
                }
            };

            exit(status);
        });

        let mut state = State::default();
        let mut line = String::new();

        // Wait for first line of input before we switch to the alt screen.
        // This helps in situations like `cargo run firmware | cargo run visualizer`,
        // where the firmware is still building (and hence writing to the screen),
        // so it doesn't stomp on the graph.
        if stdin.read_line(&mut line)? == 0 {
            // input stream closed
            return Ok(());
        }

        // We've seen our first line of input--switch to the alt screen and start rendering.
        enable_raw_mode()?;
        defer! { let _ = disable_raw_mode(); }
        execute!(&stdout, EnterAlternateScreen)?;
        defer! { let _ = execute!(&stdout, LeaveAlternateScreen); }
        terminal.hide_cursor()?;

        loop {
            // Parse line
            let redraw = parse::handle_line(&mut state, &line);

            // Redraw if requested
            match redraw {
                Redraw::No => {}
                Redraw::Yes => terminal::draw(&state, &mut terminal)?,
            }

            // Read next line
            line.clear();
            if stdin.read_line(&mut line)? == 0 {
                // input stream closed
                return Ok(());
            }
        }
    })
}
