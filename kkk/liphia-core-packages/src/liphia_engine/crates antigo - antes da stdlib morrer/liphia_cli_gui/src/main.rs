// liphia_cli_gui/src/main.rs
//
// Desktop GUI host for Liphia scripts: opens an eframe window and ticks
// the VM once per frame. Usage: liphia_cli_gui [script.lph]

mod app;
mod console;
mod file_picker;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let initial_path = std::env::args().nth(1).map(PathBuf::from);
    app::run(initial_path)
}
