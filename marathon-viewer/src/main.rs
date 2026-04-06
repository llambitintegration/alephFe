use clap::Parser;
use std::path::PathBuf;

mod level;
mod mesh;
mod render;
mod texture;
mod transfer;

#[derive(Parser)]
#[command(name = "marathon-viewer", about = "3D level viewer for Marathon scenarios")]
struct Args {
    /// Path to the map WAD file
    #[arg(long)]
    map: PathBuf,

    /// Path to the shapes WAD file
    #[arg(long)]
    shapes: PathBuf,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    if !args.map.exists() {
        eprintln!("Map file not found: {}", args.map.display());
        std::process::exit(1);
    }
    if !args.shapes.exists() {
        eprintln!("Shapes file not found: {}", args.shapes.display());
        std::process::exit(1);
    }

    if let Err(e) = render::run(args.map, args.shapes) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
