use anyhow;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal,
};
use rodio::{Decoder, OutputStream, Sink};
use std::env;
use std::fs::File;
use std::io::BufReader;

fn main() -> anyhow::Result<()> {
    // Setup terminal in raw mode to capture key presses instantly
    terminal::enable_raw_mode()?;

    // Get the music file path from command line argument
    let mut args = env::args().skip(1);
    let music_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Usage: audix <music_file>");
            terminal::disable_raw_mode()?;
            std::process::exit(1);
        }
    };

    // Set up audio output stream and sink
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    // Open music file
    let file = File::open(&music_path)?;
    let source = Decoder::new(BufReader::new(file))?;

    sink.append(source);
    sink.play();

    println!(
        "Playing {}. Press SPACE to toggle pause/play. Press 'q' to quit.",
        music_path
    );

    // Main event loop for keyboard input
    loop {
        // Wait for keyboard event
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char(' ') => {
                        if sink.is_paused() {
                            sink.play();
                            println!("Resumed");
                        } else {
                            sink.pause();
                            println!("Paused");
                        }
                    }
                    KeyCode::Char('q') => {
                        break;
                    }
                    _ => {}
                }
            }
        }

        // Exit if playback ended
        if sink.empty() {
            println!("Playback finished");
            break;
        }
    }

    terminal::disable_raw_mode()?;
    Ok(())
}
