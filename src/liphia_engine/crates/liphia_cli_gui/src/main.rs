// liphia_cli_gui/src/main.rs
use std::collections::HashSet;
use std::path::PathBuf;

use eframe::egui;
use liphia_compiler::ast::Type;
use liphia_gui_native::GuiCommand;
use liphia_pipeline::{compile_with_externals, resolve_project};
use liphia_virtual_machine::vm::{VmSession, VM};

mod console;
mod file_picker;
use console::Console;
use file_picker::FilePicker;

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
    // Desktop convenience only: if a path was passed on the command line,
    // preload it so `liphia_cli_gui script.lph` still works exactly like today.
    let args: Vec<String> = std::env::args().collect();
    let initial_path = args.get(1).map(PathBuf::from);

    let mut app = GuiApp::default();
    if let Some(path) = initial_path {
        match std::fs::read_to_string(&path) {
            Ok(source) => app.load_program(&source, Some(path)),
            Err(e) => app.console.error(format!("Failed to read {}: {e}", ".lph file")),
        }
    }

    eframe::run_native(
        "Liphia GUI",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(app))),
    )
}

/// Holds the compiled program + running VM state, only present once a
/// script has actually been loaded (either via CLI arg on desktop, or
/// via the file picker on Android/desktop alike).
struct RunningProgram {
    vm:      VM,
    session: VmSession,
}

#[derive(Default)]
struct GuiApp {
    program:     Option<RunningProgram>,
    file_picker: FilePicker,
    console:     Console,
}

impl GuiApp {
    /// Compiles `source` and, on success, replaces the currently running
    /// program with a fresh VM/session. `origin_path` is only used for
    /// resolving relative imports on desktop; pass None when the source
    /// came from a picker without a real filesystem path (Android).
    fn load_program(&mut self, source: &str, origin_path: Option<PathBuf>) {
        self.console.clear();

        let mut visited = HashSet::new();
        let stmts = match origin_path {
            // Desktop path: resolve_project can follow relative imports on disk.
            Some(path) => resolve_project(&path, &path, &mut visited),
            // Android/no-path case: single-file only for now, no import resolution.
            // If resolve_project requires a path unconditionally, a temp file
            // can be written here as a fallback — flagging this as a known gap.
            None => {
                self.console.error("Loaded without a file path: relative imports are not resolved.");
                liphia_pipeline::parse_only(source) // adjust to your actual single-source entry point
            }
        };

        let opcodes = compile_with_externals(stmts, &gui_externals());

        let mut vm = VM::new();
        liphia_core_native::register(&mut vm);
        liphia_stdlib_native::register_all(&mut vm);
        liphia_gui_native::register(&mut vm);
        let session = VmSession::new(opcodes);

        self.program = Some(RunningProgram { vm, session });
        self.console.log("Program loaded.");
    }
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Check if a background file pick finished this frame
        if let Some(source) = self.file_picker.poll() {
            self.load_program(&source, None);
        }

        liphia_gui_native::begin_frame();

        if let Some(RunningProgram { vm, session }) = &mut self.program {
            for _ in 0..8 {
                match session.tick(vm) {
                    Ok(true)  => {}
                    Ok(false) => break,
                    Err(e)    => {
                        self.console.error(format!("Runtime error: {e}"));
                        break;
                    }
                }
            }
        }

        let commands = liphia_gui_native::take_commands();

        egui::TopBottomPanel::top("toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                let label = if self.file_picker.is_picking() { "Opening..." } else { "Open File" };
                if ui.button(label).clicked() {
                    self.file_picker.open();
                }
            });
        });

        egui::TopBottomPanel::bottom("console_panel")
            .resizable(true)
            .default_height(150.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    for line in self.console.lines() {
                        let color = match line.level {
                            console::Level::Info  => ui.visuals().text_color(),
                            console::Level::Error => egui::Color32::from_rgb(220, 60, 60),
                        };
                        ui.colored_label(color, &line.text);
                    }
                });
            });

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