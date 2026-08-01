// liphia_gui_native/src/lib.rs
//
// GUI native functions for Liphia — window-toolkit agnostic on purpose.
// Natives push plain draw commands into a thread_local queue instead of
// calling egui directly (this crate has zero egui/eframe dependency).
// The host (liphia_cli_gui today) drains the queue once per frame, after
// the VM's tick round, and renders each command against the real UI.
//
// Frame pacing: egui is immediate-mode, so the .lph script must re-emit
// its draw commands every real frame. gui_next_frame() is a Suspend-based
// native (same mechanism already validated for http_accept()): it only
// returns "ready" once per real frame, consumed via begin_frame() called
// by the host at the start of each App::ui(). A script drives this with
// `while true: ...draw calls... await gui_next_frame()`.
//
// Button click feedback has one frame of latency: gui_button() enqueues
// this frame's draw command AND returns whether that id was clicked on
// the PREVIOUS frame (the host writes that via set_clicked() right after
// rendering). This matches the ergonomics script authors already expect
// from egui itself (`ui.button(...).clicked()`).
use std::cell::RefCell;
use std::collections::HashMap;
use liphia_virtual_machine::value::Value;
use liphia_virtual_machine::vm::{VmError, VmResult, VM};

#[derive(Debug, Clone)]
pub enum GuiCommand {
    Heading(String),
    Label(String),
    Separator,
    Button { id: String, text: String },
}

thread_local! {
    static COMMANDS:    RefCell<Vec<GuiCommand>>       = RefCell::new(Vec::new());
    static CLICKED:     RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
    static FRAME_READY: RefCell<bool>                  = RefCell::new(false);
}

/// Host call: drains this frame's queued draw commands for rendering.
pub fn take_commands() -> Vec<GuiCommand> {
    COMMANDS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// Host call: records whether a button id was clicked THIS frame — read
/// back by gui_button() on the next tick round.
pub fn set_clicked(id: &str, clicked: bool) {
    CLICKED.with(|c| { c.borrow_mut().insert(id.to_string(), clicked); });
}

/// Host call: signals a new real frame has started. Call once at the top
/// of App::ui(), before ticking the VM.
pub fn begin_frame() {
    FRAME_READY.with(|f| *f.borrow_mut() = true);
}

fn was_clicked(id: &str) -> bool {
    CLICKED.with(|c| c.borrow().get(id).copied().unwrap_or(false))
}

pub fn register(vm: &mut VM) {
    vm.register_native("gui_heading",    native_gui_heading);
    vm.register_native("gui_label",      native_gui_label);
    vm.register_native("gui_separator",  native_gui_separator);
    vm.register_native("gui_button",     native_gui_button);
    vm.register_native("gui_next_frame", native_gui_next_frame);
}

fn str_arg(v: &Value, ctx: &str) -> VmResult<String> {
    match v {
        Value::Str(s) => Ok(s.as_str().to_string()),
        _ => Err(VmError::new(format!("{}: argument must be str", ctx))),
    }
}

fn native_gui_heading(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 { return Err(VmError::new("gui_heading(text: str) — expected 1 argument")); }
    let text = str_arg(&args[0], "gui_heading")?;
    COMMANDS.with(|c| c.borrow_mut().push(GuiCommand::Heading(text)));
    Ok(Value::Null)
}

fn native_gui_label(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 1 { return Err(VmError::new("gui_label(text: str) — expected 1 argument")); }
    let text = str_arg(&args[0], "gui_label")?;
    COMMANDS.with(|c| c.borrow_mut().push(GuiCommand::Label(text)));
    Ok(Value::Null)
}

fn native_gui_separator(args: Vec<Value>) -> VmResult<Value> {
    if !args.is_empty() { return Err(VmError::new("gui_separator() — expected 0 arguments")); }
    COMMANDS.with(|c| c.borrow_mut().push(GuiCommand::Separator));
    Ok(Value::Null)
}

fn native_gui_button(args: Vec<Value>) -> VmResult<Value> {
    if args.len() != 2 { return Err(VmError::new("gui_button(id: str, text: str) — expected 2 arguments")); }
    let id      = str_arg(&args[0], "gui_button")?;
    let text    = str_arg(&args[1], "gui_button")?;
    let clicked = was_clicked(&id);
    COMMANDS.with(|c| c.borrow_mut().push(GuiCommand::Button { id, text }));
    Ok(Value::Bool(clicked))
}

fn native_gui_next_frame(args: Vec<Value>) -> VmResult<Value> {
    if !args.is_empty() { return Err(VmError::new("gui_next_frame() — expected 0 arguments")); }
    let ready = FRAME_READY.with(|f| {
        let mut r = f.borrow_mut();
        if *r { *r = false; true } else { false }
    });
    Ok(Value::Bool(ready))
}