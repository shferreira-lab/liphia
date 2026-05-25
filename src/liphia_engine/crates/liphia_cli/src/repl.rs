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

    println!("Welcome to the Liphia Interactive Shell! V.0.3.1");
    println!("Type 'help' for commands. Ctrl+C or 'exit' to quit.\n");

    loop {
        let in_block = is_in_block(&buffer);
        let prompt = if in_block { "... " } else { ">>> " };
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
                println!("Commands: exit, quit, reload, help");
                continue;
            }
            "reload" => {
                buffer.clear();
                declarations.clear();
                state = ReplState::new();
                println!("Session cleared.");
                continue;
            }
            _ => {}
        }

        if trimmed.is_empty() {
            if buffer.trim().is_empty() { continue; }
            if !is_in_block(&buffer) {
                execute_buffer(&buffer, &mut declarations, &mut state, &mut vm);
                buffer.clear();
            } else {
                // Still inside an open block — add blank line and keep waiting
                buffer.push('\n');
            }
            continue;
        }

        buffer.push_str(&line);

        // Single-line statement (does not open a block) — run immediately
        if !trimmed.ends_with(':') && !is_in_block(&buffer) {
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
    // Prepend all previously accumulated declarations so the compiler
    // can resolve functions and variables defined in earlier interactions
    let full_source = format!("{}{}", declarations, buffer);

    match compile_safe(&full_source, state) {
        Ok(opcodes) => {
            if let Err(e) = vm.run(opcodes) {
                eprintln!("Runtime error: {}", e);
                return;
            }
            // Only accumulate declarations, not loose expressions/prints
            if is_declaration(buffer.trim()) {
                declarations.push_str(buffer);
                if !declarations.ends_with('\n') {
                    declarations.push('\n');
                }
                // Register the new symbol in state so future TypeChecker
                // instances are aware of it without re-parsing declarations
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

    // Seed the checker with symbols from previous REPL interactions
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
        // typed declaration: "name: Type = value"
        || (first.contains(':') && first.contains('=') && !first.starts_with("if"))
}

// ── Register a new declaration into ReplState ─────────────────────────────────
//
// This is a best-effort heuristic so that the TypeChecker can be seeded
// cheaply without re-parsing the full declarations string each time.
// Full resolution still happens via the accumulated source string.
fn register_declaration(source: &str, state: &mut ReplState) {
    let first = source
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim();

    // fn name(...) -> ReturnType:
    if first.starts_with("fn ") || first.starts_with("async fn ") {
        let without_prefix = first
            .trim_start_matches("async ")
            .trim_start_matches("fn ")
            .trim();
        if let Some(paren) = without_prefix.find('(') {
            let name = without_prefix[..paren].trim().to_string();
            // Register with any/unknown types — full resolution is done
            // via the accumulated declarations source string anyway
            state.known_fns.push((name, vec![], Type::Named("any".into())));
        }
    }
    // var name = value  or  name: Type = value
    else if first.starts_with("var ") {
        let rest = first.trim_start_matches("var ").trim();
        let name = rest.split(['=', ':']).next().unwrap_or("").trim().to_string();
        if !name.is_empty() {
            state.known_vars.push((name, Type::Named("any".into())));
        }
    }
    // const name = value
    else if first.starts_with("const ") {
        let rest = first.trim_start_matches("const ").trim();
        let name = rest.split(['=', ':']).next().unwrap_or("").trim().to_string();
        if !name.is_empty() {
            state.known_vars.push((name, Type::Named("any".into())));
        }
    }
    // typed declaration: name: Type = value
    else if first.contains(':') && first.contains('=') {
        let name = first.split(':').next().unwrap_or("").trim().to_string();
        if !name.is_empty() {
            state.known_vars.push((name, Type::Named("any".into())));
        }
    }
}

// ── Detect whether the buffer has an open block ───────────────────────────────
fn is_in_block(buffer: &str) -> bool {
    let mut depth = 0i32;
    for line in buffer.lines() {
        let s = line.trim_end();
        if s.trim().is_empty() { continue; }
        let indent = line.len() - line.trim_start().len();
        // A non-empty line at column 0 that does not open a new block
        // implicitly closes whatever block was open
        if depth > 0 && indent == 0 && !s.ends_with(':') {
            depth -= 1;
        }
        if s.ends_with(':') {
            depth += 1;
        }
    }
    depth > 0
}