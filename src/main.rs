use clap;
use clap::Arg;
use clap::Command;
fn cli() -> Command {
    Command::new("audix")
        .author("Santosh")
        .about("command-line music player")
        .arg(
            Arg::new("music-dir")
                .short('d')
                .long("dir")
                .value_name("DIRECTORY")
                .required_unless_present("help"),
        )
        .arg(Arg::new("help").short("h").long("help"))
}
fn main() {}
