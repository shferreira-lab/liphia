// liphia_cli/src/repl.rs
use std::io::{self, Write};
use liphia_virtual_machine::vm::VM;
use liphia_virtual_machine::opcode::Opcode;
use liphia_compiler::ast::Type;
use liphia_compiler::lexer::Lexer;
use liphia_compiler::parser::Parser;
use liphia_compiler::bytecode::generate_bytecode;
use liphia_compiler::type_checker::TypeChecker;
use liphia_core_native;
use liphia_stdlib_native;

// ── Known symbols carried across REPL executions ─────────────────────────────
struct ReplState {
    known_vars: Vec<(String, Type)>,
    known_fns:  Vec<(String, Vec<Type>, Type)>,
}

impl ReplState {
    fn new() -> Self {
        Self { known_vars: vec![], known_fns: vec![] }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────
pub fn start() {
    let mut vm = VM::new();
    liphia_core_native::register(&mut vm);
    liphia_stdlib_native::register_all(&mut vm);

    // Accumulated source of declarations only (fn, enum, var, const)
    // so that previously defined functions are visible in future compilations
    let mut declarations = String::new();

    // Tracks declared symbols so the TypeChecker can be seeded each run
    let mut state = ReplState::new();

    let mut buffer = String::new();

    println!("Welcome to the Liphia Interactive Shell! V.0.4.0");
    println!("Type 'help' for commands. Ctrl+C or 'exit' to quit.");
    println!("Multi-line input (fn, if, while, try...) accumulates until you type 'run'.\n");

    loop {
        let in_progress = !buffer.trim().is_empty();
        let prompt = if in_progress { "... " } else { ">>> " };
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            println!("\nExiting REPL.");
            break;
        }

        let trimmed = line.trim_end();

        match trimmed {
            "exit" | "quit" => { println!("Exiting REPL."); break; }
            "help" => {
                println!("Commands:");
                println!("  run     — execute the accumulated multi-line buffer");
                println!("  reload  — clear buffer and forget all declarations");
                println!("  exit / quit — leave the REPL");
                println!("Single-line statements that don't open a block (no trailing ':')");
                println!("run immediately. Anything that opens a block (fn/if/while/try/...)");
                println!("keeps accumulating — type 'run' when you're done.");
                continue;
            }
            "reload" => {
                buffer.clear();
                declarations.clear();
                state = ReplState::new();
                println!("Session cleared.");
                continue;
            }
            "run" => {
                if buffer.trim().is_empty() {
                    println!("(nothing to run)");
                } else {
                    execute_buffer(&buffer, &mut declarations, &mut state, &mut vm);
                    buffer.clear();
                }
                continue;
            }
            _ => {}
        }

        if trimmed.is_empty() {
            // Blank line inside a multi-line buffer: just preserve spacing,
            // still waiting for an explicit 'run'.
            if in_progress {
                buffer.push('\n');
            }
            continue;
        }

        let was_fresh = !in_progress;
        buffer.push_str(&line);

        // Only auto-execute when this line is the FIRST line of a fresh
        // entry (buffer was empty before it) AND it doesn't open a block.
        // Anything that opens a block, or is typed as a continuation,
        // waits for an explicit 'run'.
        if was_fresh && !trimmed.ends_with(':') {
            execute_buffer(&buffer, &mut declarations, &mut state, &mut vm);
            buffer.clear();
        }
    }
}

// ── Execute one buffer ────────────────────────────────────────────────────────
fn execute_buffer(
    buffer:       &str,
    declarations: &mut String,
    state:        &mut ReplState,
    vm:           &mut VM,
) {
    let full_source = format!("{}{}", declarations, buffer);

    match compile_safe(&full_source, state) {
        Ok(opcodes) => {
            if let Err(e) = vm.run(opcodes) {
                eprintln!("Runtime error: {}", e);
                return;
            }
            if is_declaration(buffer.trim()) {
                declarations.push_str(buffer);
                if !declarations.ends_with('\n') {
                    declarations.push('\n');
                }
                register_declaration(buffer.trim(), state);
            }
        }
        Err(e) => eprintln!("{}", e),
    }
}

// ── Compile with awareness of previously declared symbols ─────────────────────
fn compile_safe(source: &str, state: &ReplState) -> Result<Vec<Opcode>, String> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer)
        .map_err(|e| format!("Parse error: {}", e))?;
    let ast = parser.parse()
        .map_err(|e| format!("Parse error: {}", e))?;

    let mut checker = TypeChecker::new();

    for (name, ty) in &state.known_vars {
        checker.declare_var_external(name, ty.clone());
    }
    for (name, params, ret) in &state.known_fns {
        checker.declare_fn_external(name, params.clone(), ret.clone());
    }

    checker.check(&ast)
        .map_err(|e| format!("Type error: {}", e))?;

    let program = generate_bytecode(ast)
        .map_err(|e| format!("Bytecode generation error: {}", e))?;

    Ok(program.instructions)
}

// ── Detect whether a buffer is a declaration that should be accumulated ────────
fn is_declaration(source: &str) -> bool {
    let first = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();

    first.starts_with("fn ")
        || first.starts_with("async fn ")
        || first.starts_with("enum ")
        || first.starts_with("var ")
        || first.starts_with("const ")
        || (first.contains(':') && first.contains('=') && !first.starts_with("if"))
}

// ── Register a new declaration into ReplState ─────────────────────────────────
fn register_declaration(source: &str, state: &mut ReplState) {
    let first = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();

    if first.starts_with("fn ") || first.starts_with("async fn ") {
        let without_prefix = first
            .trim_start_matches("async ")
            .trim_start_matches("fn ")
            .trim();
        if let Some(paren) = without_prefix.find('(') {
            let name = without_prefix[..paren].trim().to_string();
            state.known_fns.push((name, vec![], Type::Named("any".into())));
        }
    }
    else if first.starts_with("var ") {
        let rest = first.trim_start_matches("var ").trim();
        let name = rest.split(['=', ':']).next().unwrap_or("").trim().to_string();
        if !name.is_empty() {
            state.known_vars.push((name, Type::Named("any".into())));
        }
    }
    else if first.starts_with("const ") {
        let rest = first.trim_start_matches("const ").trim();
        let name = rest.split(['=', ':']).next().unwrap_or("").trim().to_string();
        if !name.is_empty() {
            state.known_vars.push((name, Type::Named("any".into())));
        }
    }
    else if first.contains(':') && first.contains('=') {
        let name = first.split(':').next().unwrap_or("").trim().to_string();
        if !name.is_empty() {
            state.known_vars.push((name, Type::Named("any".into())));
        }
    }
}