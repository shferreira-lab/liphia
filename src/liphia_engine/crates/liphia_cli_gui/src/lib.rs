// liphia_cli_gui/src/lib.rs
//
// All app logic lives here so it can be compiled two ways from the same
// source: as a cdylib for Android (entry point: android_main) and as an
// rlib linked into the thin desktop binary in main.rs (entry point: run()).

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

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
        ("gui_heading", vec![Type::Str], Type::Null),
        ("gui_label", vec![Type::Str], Type::Null),
        ("gui_separator", vec![], Type::Null),
        ("gui_button", vec![Type::Str, Type::Str], Type::Bool),
        ("gui_next_frame", vec![], Type::Bool),
    ]
}

/// Holds the compiled program + running VM state, only present once a
/// script has actually been loaded (either via CLI arg on desktop, or
/// via the file picker on Android/desktop alike).
struct RunningProgram {
    vm: VM,
    session: VmSession,
}

#[derive(Default)]
pub struct GuiApp {
    program: Option<RunningProgram>,
    file_picker: FilePicker,
    console: Console,
}

impl GuiApp {
    /// Compiles `source` and, on success, replaces the currently running
    /// program with a fresh VM/session. `origin_path` is only used for
    /// resolving relative imports on desktop; pass None when the source
    /// came from a picker without a real filesystem path (Android).
    pub fn load_program(&mut self, source: &str, origin_path: Option<PathBuf>) {
        self.console.clear();

        let resolved_path = match origin_path {
            Some(path) => path,
            None => {
                let tmp = std::env::temp_dir().join("liphia_picked.lph");
                if let Err(e) = std::fs::write(&tmp, source) {
                    self.console
                        .error(format!("Failed to stage picked file: {e}"));
                    return;
                }
                tmp
            }
        };

        let mut visited = HashSet::new();

        let stmts = match resolve_project(&resolved_path, &resolved_path, &mut visited) {
            Ok(s) => s,
            Err(e) => {
                self.console.error(e);
                return;
            }
        };

        let opcodes = match compile_with_externals(stmts, &gui_externals()) {
            Ok(o) => o,
            Err(e) => {
                self.console.error(e);
                return;
            }
        };

        let mut vm = VM::new();
        liphia_core_native::register(&mut vm);
        liphia_stdlib_native::register_all(&mut vm);
        liphia_gui_native::register(&mut vm);

        // Route print() into the in-app console instead of stdout — this app has
        // no visible terminal, especially on Android.
        let console_for_vm = self.console.clone();
        vm.set_output_hook(Box::new(move |line: &str| {
            console_for_vm.log(line.to_string());
        }));

        let session = VmSession::new(opcodes);

        self.program = Some(RunningProgram { vm, session });
        self.console.log("Program loaded.");
    }
}

impl eframe::App for GuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(source) = self.file_picker.poll() {
            self.load_program(&source, None);
        }

        liphia_gui_native::begin_frame();

        if let Some(RunningProgram { vm, session }) = &mut self.program {
            for _ in 0..8 {
                match session.tick(vm) {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(e) => {
                        self.console.error(format!("Runtime error: {e}"));
                        break;
                    }
                }
            }
        }

        let commands = liphia_gui_native::take_commands();

        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                let label = if self.file_picker.is_picking() {
                    "Opening..."
                } else {
                    "Open File"
                };
                if ui.button(label).clicked() {
                    self.file_picker.open();
                }
            });
        });

        egui::Panel::bottom("console_panel")
            .resizable(true)
            .default_size(150.0)
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for line in self.console.lines() {
                            let color = match line.level {
                                console::Level::Info => ui.visuals().text_color(),
                                console::Level::Error => egui::Color32::from_rgb(220, 60, 60),
                            };
                            ui.colored_label(color, &line.text);
                        }
                    });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            for cmd in commands {
                match cmd {
                    GuiCommand::Heading(text) => {
                        ui.heading(text);
                    }
                    GuiCommand::Label(text) => {
                        ui.label(text);
                    }
                    GuiCommand::Separator => {
                        ui.separator();
                    }
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

/// Desktop entry point, called from main.rs.
pub fn run(initial_path: Option<PathBuf>) -> eframe::Result<()> {
    let mut app = GuiApp::default();
    if let Some(path) = initial_path {
        match std::fs::read_to_string(&path) {
            Ok(source) => app.load_program(&source, Some(path)),
            Err(e) => eprintln!("Failed to read file: {e}"),
        }
    }

    eframe::run_native(
        "Liphia GUI",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(app))),
    )
}

/// Android entry point, called by the Android runtime via cargo-apk.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };

    eframe::run_native(
        "Liphia GUI",
        options,
        Box::new(|_cc| Ok(Box::new(GuiApp::default()))),
    )
    .unwrap();
}
