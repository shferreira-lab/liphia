// src/repl.rs

use std::io::{self, Write};
use liphia_virtual_machine::vm::VM;
use liphia_virtual_machine::opcode::Opcode;
use liphia_compiler::lexer::Lexer;
use liphia_compiler::parser::Parser;
use liphia_compiler::bytecode::generate_bytecode;
use liphia_compiler::type_checker::TypeChecker;
use liphia_core_native; 
use liphia_stdlib_native; 

/// Shell LPH like Python
pub fn start() {
    let mut vm = VM::new();
    liphia_core_native::register(&mut vm);
    liphia_stdlib_native::register_all(&mut vm);

    let mut buffer = String::new();
    let mut indent_level = 0;

    println!("Welcome to the Liphia Interactive Shell! V.0.3.1");
    println!("Type 'help' for commands. Ctrl+C or 'exit' to quit.\n");

    loop {
        let prompt = if indent_level == 0 { ">>> " } else { "... " };
        print!("{}", prompt);
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            println!("\nExiting REPL.");
            break;
        }

        let trimmed = line.trim_end();

        // Special commands of REPL
        match trimmed {
            "exit" | "quit" => {
                println!("Exiting REPL.");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  exit, quit  - exit the REPL");
                println!("  reload      - clear current buffer");
                println!("  help        - show this message");
                continue;
            }
            "reload" => {
                buffer.clear();
                indent_level = 0;
                println!("Buffer cleared.");
                continue;
            }
            _ => {}
        }

        // Controle simples de indentação estilo Python
        if trimmed.ends_with(":") {
            indent_level += 1;
        } else if trimmed.is_empty() && indent_level > 0 {
            indent_level -= 1;
        }

        buffer.push_str(&line);

        // Executa apenas quando bloco fechado
        if indent_level == 0 && !buffer.trim().is_empty() {
            match compile_safe(&buffer) {
                Ok(opcodes) => {
                    if let Err(e) = vm.run(opcodes) {
                        eprintln!("Runtime error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
            buffer.clear();
        }
    }
}

/// Compilação segura: captura parse/type/bytecode errors sem encerrar o REPL
fn compile_safe(source: &str) -> Result<Vec<Opcode>, String> {
    // 1. Lex + Parse
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer)
        .map_err(|e| format!("Parse error: {}", e))?;
    let ast = parser.parse()
        .map_err(|e| format!("Parse error: {}", e))?;

    // 2. Type check
    let mut checker = TypeChecker::new();
    checker.check(&ast)
        .map_err(|e| format!("Type error: {}", e))?;

    // 3. Generate bytecode
    let program = generate_bytecode(ast)
        .map_err(|e| format!("Bytecode generation error: {}", e))?;

    Ok(program.instructions)
}