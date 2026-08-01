// liphia_cli_gui/src/main.rs
use std::collections::HashSet;
use std::path::PathBuf;
use std::process;

use eframe::egui;
use liphia_compiler::ast::Type;
use liphia_gui_native::GuiCommand;
use liphia_pipeline::{compile_with_externals, resolve_project};
use liphia_virtual_machine::vm::{VmSession, VM};

fn gui_externals() -> Vec<(&'static str, Vec<Type>, Type)> {
    vec![
        ("gui_heading",    vec![Type::Str],           Type::Null),
        ("gui_label",      vec![Type::Str],           Type::Null),
        ("gui_separator",  vec![],                    Type::Null),
        ("gui_button",     vec![Type::Str, Type::Str], Type::Bool),
        ("gui_next_frame", vec![],                    Type::Bool),
    ]
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let source_path = args.get(1).map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("usage: liphia_cli_gui <script.lph>");
        process::exit(1);
    });

    let mut visited = HashSet::new();
    let stmts   = resolve_project(&source_path, &source_path, &mut visited);
    let opcodes = compile_with_externals(stmts, &gui_externals());

    let mut vm = VM::new();
    liphia_core_native::register(&mut vm);
    liphia_stdlib_native::register_all(&mut vm);
    liphia_gui_native::register(&mut vm);
    let session = VmSession::new(opcodes);

    eframe::run_native(
        "Liphia GUI",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(GuiApp { vm, session }))),
    )
}

struct GuiApp {
    vm:      VM,
    session: VmSession,
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        liphia_gui_native::begin_frame();

        for _ in 0..8 {
            match self.session.tick(&mut self.vm) {
                Ok(true)  => {}
                Ok(false) => break,
                Err(e)    => { eprintln!("runtime error: {}", e); break; }
            }
        }

        let commands = liphia_gui_native::take_commands();

        egui::CentralPanel::default().show_inside(ui, |ui| {
            for cmd in commands {
                match cmd {
                    GuiCommand::Heading(text) => { ui.heading(text); }
                    GuiCommand::Label(text)   => { ui.label(text); }
                    GuiCommand::Separator     => { ui.separator(); }
                    GuiCommand::Button { id, text } => {
                        let clicked = ui.button(text).clicked();
                        liphia_gui_native::set_clicked(&id, clicked);
                    }
                }
            }
        });

        ui.ctx().request_repaint();
    }
}