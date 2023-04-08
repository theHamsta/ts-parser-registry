use anyhow::{bail, Context};
use clap::Parser;
use log::info;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(clap::Parser)]
#[clap(author, version)]
struct Args {
    /// Path to parser library root
    #[clap(long, default_value = ".")]
    grammar_path: PathBuf,
    /// Path where intermediate artifacts should be placed
    #[clap(short, long, default_value = "./artifacts")]
    artifact_path: PathBuf,
    /// Grammar name
    #[clap(short, long)]
    grammar_name: String,

    /// Compilation target
    #[clap(short, long, default_value = "x86_64-unknown-linux-gnu")]
    target: String,
}

fn compile_c_dynlib(
    src_dir: &Path,
    dst_dir: &Path,
    dst_name: &str,
    target: &str,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst_dir)?;
    let parser_path = src_dir.join("parser.c");
    let c_scanner = src_dir.join("scanner.c");
    let cpp_scanner = src_dir.join("scanner.cc");
    let library_path = dst_dir.join(dst_name);

    let scanner_path = if c_scanner.is_file() {
        Some(c_scanner)
    } else if cpp_scanner.is_file() {
        Some(cpp_scanner)
    } else {
        None
    };

    let header_path = src_dir;
    let mut config = cc::Build::new();
    config
        .cpp(true)
        .opt_level(2)
        .cargo_metadata(false)
        .target(target)
        .host(target);
    let compiler = config.get_compiler();
    let mut command = Command::new(compiler.path());
    for (key, value) in compiler.env() {
        command.env(key, value);
    }

    if cfg!(windows) {
        command.args(&["/nologo", "/LD", "/I"]).arg(header_path);
        command.arg("/O2");
        command.arg(parser_path);
        if let Some(scanner_path) = scanner_path.as_ref() {
            command.arg(scanner_path);
        }
        command
            .arg("/link")
            .arg(format!("/out:{}", library_path.to_str().unwrap()));
    } else {
        command
            .arg("-shared")
            .arg("-fPIC")
            .arg("-fno-exceptions")
            .arg("-g")
            .arg("-I")
            .arg(header_path)
            .arg("-o")
            .arg(&library_path)
            .arg("-O2");

        if let Some(scanner_path) = scanner_path.as_ref() {
            if scanner_path.extension() == Some("c".as_ref()) {
                command.arg("-xc").arg("-std=c99").arg(scanner_path);
            } else {
                command.arg(scanner_path);
            }
        }
        command.arg("-xc").arg(parser_path);
    }

    let output = command
        .output()
        .with_context(|| "Failed to execute C compiler")?;
    if !output.status.success() {
        bail!(
            "Parser compilation failed.\nStdout: {}\nStderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = command
        .output()
        .with_context(|| "Failed to execute C compiler")?;
    if !output.status.success() {
        bail!(
            "Parser compilation failed.\nStdout: {}\nStderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

fn generate_artifacts(args: Args) -> anyhow::Result<()> {
    std::fs::create_dir_all(&args.artifact_path)?;
    let generate_output = Command::new("tree-sitter")
        .args([
            "generate",
            &args.grammar_path.join("grammar.js").to_string_lossy(),
        ])
        .current_dir(&args.artifact_path)
        .output()?;
    if !generate_output.status.success() {
        bail!(
            "Failed to run \"tree-sitter generate\": {}",
            String::from_utf8_lossy(&generate_output.stderr)
        );
    }
    info!("Finished \"tree-sitter generate\"");

    let c_dynlib_path = args.artifact_path.join("c-dynlib");
    compile_c_dynlib(
        &args.grammar_path.join("src"),
        &c_dynlib_path,
        &format!("{}.so", args.grammar_name),
        &args.target,
    )?;
    info!("Finished compilation of dynamic C library");

    Ok(())
}

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();
    let args = Args::parse();

    generate_artifacts(args)?;

    Ok(())
}
