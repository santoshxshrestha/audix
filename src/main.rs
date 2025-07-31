use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(required = true)]
    file_path: PathBuf,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let file = File::open(&args.file_path)?;
    let source = Decoder::new(BufReader::new(file))?;

    sink.append(source);
    println!(
        " Playing: {}. Press Space to pause/play, 'q' to quit.",
        args.file_path.display()
    );

    enable_raw_mode()?;

    loop {
        if event::poll(Duration::from_millis(1000))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char(' ') => {
                        if sink.is_paused() {
                            sink.play();
                            println!(" Resumed");
                        } else {
                            sink.pause();
                            println!(" Paused");
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Esc => {
                        println!(" Exiting audix. Goodbye!");
                        break;
                    }
                    _ => {}
                }
            }
        }

        if sink.empty() {
            println!("\nPlayback finished.");
            break;
        }
    }

    disable_raw_mode()?;
    Ok(())
}
