# Audix
A simple command-line audio player written in Rust.
Usage

This program plays an audio file from the command line. You must provide the path to the file as an argument.
Building
To build the project, ensure you have Rust and Cargo installed. Then, run the following command in the project directory:
```bash
cargo build --release

```
---

## Running
Run the program from the terminal with the path to your audio file:
```bash
cargo run --release -- <file_path>

```

## By the use of cargo

you can directly install the tool by the use of the cargo 
```bash
cargo install audix
```

then 
```
bash
audix <file_path>
```

Replace <file_path> with the actual path to your .wav, .mp3, or other supported audio file.
---
# Controls
Once the audio starts playing, you can control it with the following keyboard shortcuts:
Spacebar: Pause and resume playback.
'q' or Esc: Quit the application.
