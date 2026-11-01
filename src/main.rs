mod app;
mod protocol;
mod sync;
mod ui;

use std::io;

use anyhow::Result;
use clap::{Parser, Subcommand};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use app::App;

#[derive(Debug, Parser)]
#[command(name = "soundlane", version, about = "Listen together from the terminal")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a room and generate invitations from the TUI.
    Host {
        #[arg(long, default_value = "host")]
        name: String,
        #[arg(long, default_value_t = 128)]
        bitrate: u32,
        #[arg(long)]
        stun: Option<String>,
        #[arg(long)]
        turn: Option<String>,
        /// A source label shown to participants, for example "Spotify".
        #[arg(long, default_value = "system audio")]
        source: String,
    },
    /// Join a room by pasting an invitation into the TUI.
    Join {
        #[arg(long, default_value = "guest")]
        name: String,
        #[arg(long)]
        stun: Option<String>,
        #[arg(long)]
        turn: Option<String>,
    },
    /// Print the audio devices detected by the configured audio adapter.
    Devices,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if matches!(&cli.command, Command::Devices) {
        println!("Audio-device discovery is provided by the platform capture adapter.");
        println!("Run `soundlane host` to select a source in the TUI.");
        return Ok(());
    }

    let app = match cli.command {
        Command::Host { name, bitrate, stun, turn, source } => {
            App::host(name, source, bitrate, stun, turn)
        }
        Command::Join { name, stun, turn } => App::guest(name, stun, turn),
        Command::Devices => unreachable!(),
    };
    run(app)
}

fn run(mut app: App) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = (|| -> Result<()> {
        while !app.should_quit {
            terminal.draw(|frame| ui::draw(frame, &app))?;
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => app.quit(),
                            KeyCode::Esc => app.clear_input(),
                            KeyCode::Backspace => app.backspace(),
                            KeyCode::Enter => app.submit_input(),
                            KeyCode::Char(ch) => app.push_input(ch),
                            _ => {}
                        }
                    }
                }
            }
            app.tick();
        }
        Ok(())
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    result
}
