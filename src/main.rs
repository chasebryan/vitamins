use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use vitamins::compile_to_latex;

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();

    if args.is_empty() || args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return Ok(());
    }

    let command = match args[0].as_str() {
        "check" | "compile" | "emit" => args[0].clone(),
        _ => "compile".to_string(),
    };

    let rest = if matches!(args[0].as_str(), "check" | "compile" | "emit") {
        &args[1..]
    } else {
        &args[..]
    };

    let (input, output) = parse_paths(rest)?;
    let source = fs::read_to_string(&input)
        .map_err(|err| format!("failed to read {}: {err}", input.display()))?;
    let latex = compile_to_latex(&source).map_err(|err| err.to_string())?;

    match command.as_str() {
        "check" => {
            println!("ok: {}", input.display());
            Ok(())
        }
        "emit" => {
            print!("{latex}");
            Ok(())
        }
        "compile" => {
            let output = output.unwrap_or_else(|| default_output_path(&input));
            fs::write(&output, latex)
                .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
            println!("wrote {}", output.display());
            Ok(())
        }
        _ => unreachable!(),
    }
}

fn parse_paths(args: &[String]) -> Result<(PathBuf, Option<PathBuf>), String> {
    let mut input = None;
    let mut output = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "-o" | "--output" => {
                index += 1;
                let path = args
                    .get(index)
                    .ok_or_else(|| "missing path after -o/--output".to_string())?;
                output = Some(PathBuf::from(path));
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option: {value}"));
            }
            value => {
                if input.is_some() {
                    return Err(format!("unexpected extra input: {value}"));
                }
                input = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let input = input.ok_or_else(|| "missing input .vit file".to_string())?;
    Ok((input, output))
}

fn default_output_path(input: &Path) -> PathBuf {
    let mut output = input.to_path_buf();
    output.set_extension("tex");
    output
}

fn print_help() {
    println!(
        "Vitamins compiler\n\n\
Usage:\n  \
vitamins compile <input.vit> [-o output.tex]\n  \
vitamins check <input.vit>\n  \
vitamins emit <input.vit>\n\n\
With no explicit command, `compile` is assumed."
    );
}
