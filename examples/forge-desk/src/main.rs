mod app;
mod state;
mod view;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = false;
    let mut data = PathBuf::from(".forge-desk-data");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--agent" => agent = true,
            "--data" => data = PathBuf::from(args.next().ok_or("--data requires a directory")?),
            "--help" | "-h" => {
                println!("Forge Desk — a desktop starter using forge-ui 0.1.0 from crates.io\n\n  --data PATH   Dedicated NEDB directory (default .forge-desk-data)\n  --agent       Enable local JSON-lines control on stdin/stdout\n  --help        This help\n\nStart with cargo run --release. Read README.md to customize.");
                return Ok(());
            }
            _ => return Err(format!("Unknown argument: {arg}; use --help").into()),
        }
    }
    let desk = app::Desk::open(&data).map_err(std::io::Error::other)?;
    forge_ui::run(
        desk,
        forge_ui::WindowOptions {
            title: "Forge Desk — Your local workbench".into(),
            width: 1100.0,
            height: 820.0,
            agent,
        },
    )
}
