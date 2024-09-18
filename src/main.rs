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

    let _campaigns =
        game::campaigns::read_compaign_defs(&String::from_utf8_lossy(config_file.as_ref()));

    for c in _campaigns
        .iter()
        .map(|c| format!("{} ({})", c.title, c.base_name))
    {
        println!("campaign: {}", c);
    }

    let image_defs_file = vfs.open("config/image_defs.txt").unwrap();
    let images = game::images::read_image_defs(&String::from_utf8_lossy(image_defs_file.as_ref()));

    // println!("images: {:#?}", images);
}
