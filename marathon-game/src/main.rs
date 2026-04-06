use std::path::PathBuf;

use clap::Parser;

mod level;
mod mesh;
mod render;
mod sprites;
mod texture;

#[derive(Parser)]
#[command(name = "marathon-game", about = "Marathon game engine")]
struct Args {
    /// Path to the map WAD file
    #[arg(long)]
    map: PathBuf,

    /// Path to the shapes WAD file
    #[arg(long)]
    shapes: PathBuf,

    /// Path to the sounds WAD file (optional)
    #[arg(long)]
    sounds: Option<PathBuf>,

    /// Starting level index (default: 0)
    #[arg(long, default_value_t = 0)]
    level: usize,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    if !args.map.exists() {
        eprintln!("Error: map file not found: {}", args.map.display());
        std::process::exit(1);
    }
    if !args.shapes.exists() {
        eprintln!("Error: shapes file not found: {}", args.shapes.display());
        std::process::exit(1);
    }
    if let Some(ref sounds) = args.sounds {
        if !sounds.exists() {
            eprintln!("Error: sounds file not found: {}", sounds.display());
            std::process::exit(1);
        }
    }

    if let Err(e) = render::run(args.map, args.shapes, args.sounds, args.level) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
