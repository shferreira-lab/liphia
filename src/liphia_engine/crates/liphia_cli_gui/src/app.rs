// liphia_cli_gui/src/app.rs
//
// App state and egui rendering. The VM runs tick-by-tick inside the
// window's own event loop (see VmSession), so neither blocks the other.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::rc::Rc;

use eframe::egui;
use liphia_compiler::ast::Type;
use liphia_gui_native::GuiCommand;
use liphia_manifest::RunPlan;
use liphia_pipeline::{compile_with_externals, resolve_project_with, PackageRoots, Visibility};
use liphia_virtual_machine::vm::{VmSession, VM};

use crate::console::{self, Console};
use crate::file_picker::FilePicker;

fn gui_externals() -> Vec<(&'static str, Vec<Type>, Type)> {
    vec![
        ("gui_heading", vec![Type::Str], Type::Null),
        ("gui_label", vec![Type::Str], Type::Null),
        ("gui_separator", vec![], Type::Null),
        ("gui_button", vec![Type::Str, Type::Str], Type::Bool),
        ("gui_next_frame", vec![], Type::Bool),
    ]
}

// The pipeline's view of a run plan: importable packages plus per-folder
// visibility (same conversion as the CLI's).
fn package_roots(plan: &RunPlan) -> PackageRoots {
    let visibility = plan
        .scopes
        .iter()
        .map(|s| Visibility {
            dir: s.dir.clone(),
            label: s.label.clone(),
            allowed: s.allowed.iter().cloned().collect(),
        })
        .collect();
    PackageRoots::scoped(plan.scope.clone(), plan.packages.clone()).with_visibility(visibility)
}

/// Holds the compiled program + running VM state, only present once a
/// script has actually been loaded (either via CLI arg on desktop, or
/// via the file picker).
struct RunningProgram {
    vm: VM,
    session: VmSession,
}

/// Bridge between the script's input() and the console's text field.
/// The VM's input hook sets `waiting` while it has no answer; the UI shows
/// the field while `waiting` is true and stores the typed line in `answer`.
#[derive(Default)]
struct InputState {
    waiting: bool,
    answer: Option<String>,
}

#[derive(Default)]
pub struct GuiApp {
    program: Option<RunningProgram>,
    file_picker: FilePicker,
    console: Console,
    input: Rc<RefCell<InputState>>,
    input_text: String,
}

impl GuiApp {
    /// Compiles `source` and, on success, replaces the currently running
    /// program with a fresh VM/session. `origin_path` is only used for
    /// resolving relative imports; pass None when the source came from the
    /// file picker, which only returns the file's bytes, not its path.
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

        // A file inside a project or workspace imports only what its member
        // declares and loads only those natives; a picked file with no path
        // (or outside any project) keeps the older global lookup.
        let plan = match RunPlan::for_file(&resolved_path) {
            Ok(plan) => plan,
            Err(e) => {
                self.console.error(e);
                return;
            }
        };
        let roots = plan.as_ref().map(package_roots).unwrap_or_default();

        let mut visited = HashSet::new();

        let stmts = match resolve_project_with(&resolved_path, &resolved_path, &mut visited, &roots) {
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
        liphia_gui_native::register(&mut vm);
        // Native packages (db, num, stats, learn) ship as prebuilt libraries
        // under liphia_modules/<name>/lib/, loaded through
        // liphia_virtual_machine::external. Harmless no-op if none installed.
        match &plan {
            Some(plan) => {
                for dir in &plan.native_dirs {
                    if let Err(e) = vm.load_external_module(dir) {
                        let name = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
                        self.console.error(format!("failed to load package '{}': {}", name, e.message));
                    }
                }
            }
            None => {
                for (name, err) in vm.load_installed_external_modules("liphia_modules") {
                    self.console.error(format!("failed to load package '{}': {}", name, err.message));
                }
            }
        }

        // Route print() into the in-app console instead of stdout.
        let console_for_vm = self.console.clone();
        vm.set_output_hook(Box::new(move |line: &str| {
            console_for_vm.log(line.to_string());
        }));

        // Answer input() from the console's text field instead of stdin,
        // which the GUI does not have. Returning None makes the task yield
        // and retry, so the window keeps redrawing while it waits.
        *self.input.borrow_mut() = InputState::default();
        self.input_text.clear();
        let input_for_vm = Rc::clone(&self.input);
        vm.set_input_hook(Box::new(move || {
            let mut state = input_for_vm.borrow_mut();
            match state.answer.take() {
                Some(line) => {
                    state.waiting = false;
                    Some(line)
                }
                None => {
                    state.waiting = true;
                    None
                }
            }
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
                // Text field shown only while the script is blocked in input().
                if self.input.borrow().waiting {
                    ui.horizontal(|ui| {
                        ui.label("input:");
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut self.input_text)
                                .hint_text("type and press Enter"),
                        );
                        let enter = field.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        // Keep the cursor in the field without fighting the
                        // Enter key, which makes the field lose focus.
                        if !enter {
                            field.request_focus();
                        }
                        if enter || ui.button("Send").clicked() {
                            let line = std::mem::take(&mut self.input_text);
                            self.console.log(format!("> {}", line));
                            let mut state = self.input.borrow_mut();
                            state.answer = Some(line);
                            state.waiting = false;
                        }
                    });
                    ui.separator();
                }

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

/// Entry point, called from main.rs.
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
