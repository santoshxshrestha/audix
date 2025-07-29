use clap::{Arg, Command};
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};
use rodio::{Decoder, OutputStream, Sink};
use std::fs;
use std::io::{BufReader, stdout};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

fn cli() -> Command {
    Command::new("audix")
        .author("Santosh")
        .about("command-line music player")
        .arg(
            Arg::new("music-dir")
                .short('d')
                .long("dir")
                .value_name("DIRECTORY")
                .help("Directory containing music files")
                .required(true),
        )
        .arg(
            Arg::new("shuffle")
                .short('s')
                .long("shuffle")
                .help("Shuffle playlist")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("volume")
                .short('v')
                .long("volume")
                .value_name("LEVEL")
                .help("Set volume (0.0 to 1.0)")
                .default_value("0.7"),
        )
}

#[derive(Clone)]
struct PlayerState {
    current_track: usize,
    is_playing: bool,
    volume: f32,
    position: Duration,
    duration: Duration,
    track_name: String,
    playlist_len: usize,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current_track: 0,
            is_playing: false,
            volume: 0.7,
            position: Duration::new(0, 0),
            duration: Duration::new(0, 0),
            track_name: String::new(),
            playlist_len: 0,
        }
    }
}

fn scan_music_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut music_files = Vec::new();
    let supported_extensions = ["mp3", "wav", "flac", "ogg", "m4a"];

    fn scan_recursive(
        dir: &Path,
        music_files: &mut Vec<std::path::PathBuf>,
        extensions: &[&str],
    ) -> Result<(), Box<dyn std::error::Error>> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                scan_recursive(&path, music_files, extensions)?;
            } else if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str() {
                        if extensions.contains(&ext_str.to_lowercase().as_str()) {
                            music_files.push(path);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    scan_recursive(dir, &mut music_files, &supported_extensions)?;

    if music_files.is_empty() {
        return Err("No supported audio files found in directory".into());
    }

    music_files.sort();
    Ok(music_files)
}

fn draw_visualizer(width: usize) -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let mut bars = String::new();

    for _ in 0..width {
        let height = rng.gen_range(1..=8);
        let bar = match height {
            1 => "▁",
            2 => "▂",
            3 => "▃",
            4 => "▄",
            5 => "▅",
            6 => "▆",
            7 => "▇",
            _ => "█",
        };
        bars.push_str(bar);
    }
    bars
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

fn draw_progress_bar(position: Duration, duration: Duration, width: usize) -> String {
    if duration.as_secs() == 0 {
        return "─".repeat(width);
    }

    let progress = position.as_secs_f64() / duration.as_secs_f64();
    let filled = (progress * width as f64) as usize;
    let remaining = width.saturating_sub(filled);

    format!("{}{}", "━".repeat(filled.min(width)), "─".repeat(remaining))
}

fn draw_volume_bar(volume: f32, width: usize) -> String {
    let filled = (volume * width as f32) as usize;
    let remaining = width.saturating_sub(filled);

    format!("{}{}", "█".repeat(filled.min(width)), "░".repeat(remaining))
}

fn display_ui(state: &PlayerState) -> Result<(), Box<dyn std::error::Error>> {
    let mut stdout = stdout();

    execute!(
        stdout,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;

    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print("╔══════════════════════════════════════════════════════╗\n"),
        Print("║                  Audix Music Player                 ║\n"),
        Print("╚══════════════════════════════════════════════════════╝\n"),
        ResetColor
    )?;

    execute!(
        stdout,
        SetForegroundColor(Color::Yellow),
        Print(format!("  Now Playing: {}\n", state.track_name)),
        ResetColor
    )?;

    execute!(
        stdout,
        SetForegroundColor(Color::Green),
        Print(format!("  {}\n", draw_visualizer(50))),
        ResetColor
    )?;

    let progress_bar = draw_progress_bar(state.position, state.duration, 50);
    execute!(
        stdout,
        Print(format!(
            "  {} {} / {}\n",
            progress_bar,
            format_duration(state.position),
            format_duration(state.duration)
        ))
    )?;

    let status_icon = if state.is_playing { "" } else { "" };
    execute!(
        stdout,
        SetForegroundColor(Color::Blue),
        Print(format!(
            "  {} Track {} of {}\n",
            status_icon,
            state.current_track + 1,
            state.playlist_len
        )),
        ResetColor
    )?;

    let volume_bar = draw_volume_bar(state.volume, 20);
    execute!(
        stdout,
        Print(format!(
            "   Volume: {} {:.0}%\n",
            volume_bar,
            state.volume * 100.0
        ))
    )?;

    execute!(stdout, Print("\n"))?;
    execute!(
        stdout,
        SetForegroundColor(Color::DarkGrey),
        Print("  ╔═══════════════════════════════════════════════════════╗\n"),
        Print("  ║                      Controls                         ║\n"),
        Print("  ║  [Space] Play/Pause    [←/→] Previous/Next           ║\n"),
        Print("  ║  [↑/↓] Volume          [r] Restart Track             ║\n"),
        Print("  ║  [q] Quit              [s] Shuffle Toggle            ║\n"),
        Print("  ╚═══════════════════════════════════════════════════════╝\n"),
        ResetColor
    )?;

    Ok(())
}

fn get_track_duration(_file_path: &Path) -> Duration {
    Duration::from_secs(180)
}

fn play_music(
    files: &[std::path::PathBuf],
    mut shuffle: bool,
    initial_volume: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create audio output - compatible with rodio 0.19+
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| format!("Failed to create output stream: {}", e))?;

    // Create sink - using the correct method for your rodio version
    let sink =
        Sink::try_new(&stream_handle).map_err(|e| format!("Failed to create sink: {}", e))?;

    let mut playlist = files.to_vec();
    if shuffle {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        playlist.shuffle(&mut rng);
    }

    sink.set_volume(initial_volume);

    let state = Arc::new(Mutex::new(PlayerState {
        volume: initial_volume,
        playlist_len: playlist.len(),
        ..Default::default()
    }));

    terminal::enable_raw_mode()?;

    let mut current_index = 0;
    let mut last_update = Instant::now();
    let mut track_start_time = Instant::now();

    loop {
        if sink.empty() && current_index < playlist.len() {
            let file_path = &playlist[current_index];
            let track_name = file_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            match std::fs::File::open(file_path) {
                Ok(file) => match Decoder::new(BufReader::new(file)) {
                    Ok(source) => {
                        let duration = get_track_duration(file_path);

                        {
                            let mut state_lock = state.lock().unwrap();
                            state_lock.current_track = current_index;
                            state_lock.track_name = track_name;
                            state_lock.duration = duration;
                            state_lock.position = Duration::new(0, 0);
                            state_lock.is_playing = true;
                        }

                        sink.append(source);
                        track_start_time = Instant::now();
                        current_index += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed to decode {}: {}", file_path.display(), e);
                        current_index += 1;
                        continue;
                    }
                },
                Err(e) => {
                    eprintln!("Failed to open {}: {}", file_path.display(), e);
                    current_index += 1;
                    continue;
                }
            }
        }

        if current_index >= playlist.len() && sink.empty() {
            current_index = 0;
            if !playlist.is_empty() {
                continue;
            } else {
                break;
            }
        }

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                let mut should_skip = false;
                let mut should_previous = false;
                let mut should_restart = false;
                let mut should_shuffle = false;

                {
                    let mut state_lock = state.lock().unwrap();

                    match code {
                        KeyCode::Char(' ') => {
                            if state_lock.is_playing {
                                sink.pause();
                                state_lock.is_playing = false;
                            } else {
                                sink.play();
                                state_lock.is_playing = true;
                            }
                        }
                        KeyCode::Char('q') => break,
                        KeyCode::Left => {
                            should_previous = true;
                        }
                        KeyCode::Right => {
                            should_skip = true;
                        }
                        KeyCode::Char('r') => {
                            should_restart = true;
                        }
                        KeyCode::Char('s') => {
                            should_shuffle = true;
                        }
                        KeyCode::Up => {
                            state_lock.volume = (state_lock.volume + 0.05).min(1.0);
                            sink.set_volume(state_lock.volume);
                        }
                        KeyCode::Down => {
                            state_lock.volume = (state_lock.volume - 0.05).max(0.0);
                            sink.set_volume(state_lock.volume);
                        }
                        _ => {}
                    }
                }

                if should_skip {
                    sink.stop();
                } else if should_previous {
                    if current_index > 1 {
                        current_index -= 2;
                        sink.stop();
                    } else if current_index == 1 {
                        current_index = playlist.len() - 1;
                        sink.stop();
                    }
                } else if should_restart {
                    if current_index > 0 {
                        current_index -= 1;
                        sink.stop();
                    }
                } else if should_shuffle {
                    shuffle = !shuffle;
                    if shuffle {
                        use rand::seq::SliceRandom;
                        let mut rng = rand::thread_rng();
                        let current_track = if current_index > 0 {
                            playlist[current_index - 1].clone()
                        } else {
                            playlist[0].clone()
                        };
                        playlist.shuffle(&mut rng);
                        if let Some(pos) = playlist.iter().position(|x| x == &current_track) {
                            playlist.swap(0, pos);
                            current_index = 1;
                        }
                    } else {
                        playlist = files.to_vec();
                        playlist.sort();
                    }
                }
            }
        }

        if last_update.elapsed() >= Duration::from_millis(100) {
            {
                let mut state_lock = state.lock().unwrap();
                if state_lock.is_playing {
                    state_lock.position = track_start_time.elapsed().min(state_lock.duration);
                }
            }
            last_update = Instant::now();
        }

        {
            let state_lock = state.lock().unwrap();
            if let Err(e) = display_ui(&state_lock) {
                eprintln!("Display error: {}", e);
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    terminal::disable_raw_mode()?;
    execute!(
        stdout(),
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;
    println!("Thanks for using Audix Music Player! Goodbye!");

    Ok(())
}

fn main() {
    let matches = cli().get_matches();

    let music_dir = matches.get_one::<String>("music-dir").unwrap();
    let shuffle = matches.get_flag("shuffle");
    let volume: f32 = matches
        .get_one::<String>("volume")
        .unwrap()
        .parse::<f32>()
        .unwrap_or(0.7f32)
        .clamp(0.0f32, 1.0f32);

    let dir_path = Path::new(music_dir);

    if !dir_path.exists() {
        eprintln!("Error: Directory '{}' does not exist", music_dir);
        std::process::exit(1);
    }

    if !dir_path.is_dir() {
        eprintln!("Error: '{}' is not a directory", music_dir);
        std::process::exit(1);
    }

    match scan_music_files(dir_path) {
        Ok(files) => {
            println!("Found {} audio files", files.len());
            if let Err(e) = play_music(&files, shuffle, volume) {
                eprintln!("Playback error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Error scanning directory: {}", e);
            std::process::exit(1);
        }
    }
}

