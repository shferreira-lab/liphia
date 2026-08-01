// liphia_cli_gui/src/main.rs
//
// Proof-of-concept GUI host: opens a window via eframe/egui, and ticks a
// Liphia VmSession a bounded number of times per frame — proving the VM's
// scheduler and the window's own event loop coexist without either one
// blocking the other. Widget natives (gui_label/gui_button/etc, driven from
// .lph itself) come in a later slice once this architecture is confirmed.
use std::collections::HashSet;
use std::path::PathBuf;
use std::process;

use eframe::egui;
use liphia_pipeline::{compile, resolve_project};
use liphia_virtual_machine::vm::{VmSession, VM};

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let source_path = args.get(1).map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("usage: liphia_cli_gui <script.lph>");
        process::exit(1);
    });

    let mut visited = HashSet::new();
    let stmts   = resolve_project(&source_path, &source_path, &mut visited);
    let opcodes = compile(stmts);

    let mut vm = VM::new();
    liphia_core_native::register(&mut vm);
    liphia_stdlib_native::register_all(&mut vm);
    let session = VmSession::new(opcodes);

    eframe::run_native(
    "Liphia GUI (proof of concept)",
    eframe::NativeOptions::default(),
    Box::new(|_cc| {
        Ok(Box::new(GuiApp {
            vm,
            session,
            counter: 0,
            }))
        }),
        )
    }

struct GuiApp {
    vm: VM,
    session: VmSession,
    counter: i32,
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Advance the VM a bounded number of ticks per frame.
        for _ in 0..8 {
            match self.session.tick(&mut self.vm) {
                Ok(true)  => {}
                Ok(false) => break,
                Err(e)    => { eprintln!("runtime error: {}", e); break; }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Liphia GUI — proof of concept");
            ui.label("This window stays responsive while the VM ticks in the background.");
            ui.separator();
            if ui.button("click me").clicked() {
                self.counter += 1;
            }
            ui.label(format!("clicks: {}", self.counter));
        });

        ctx.request_repaint();
    }
}