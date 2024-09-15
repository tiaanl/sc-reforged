use std::path::PathBuf;

use clap::Parser;

mod engine;
mod game;

#[derive(clap::Parser)]
struct Opts {
    /// Path to the game data directory.
    /// (e.g. "C:\Program Files\Sinister Games\Shadow Comapany - Left for Dead\Data")
    path: PathBuf,
}

fn main() {
    let opts = Opts::parse();

    let vfs = engine::vfs::VirtualFileSystem::new(opts.path);
    let config_file = vfs.open("config/campaign_defs.txt").unwrap();

    println!("{}", String::from_utf8_lossy(config_file.as_ref()));
}
