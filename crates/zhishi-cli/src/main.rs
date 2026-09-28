//! zsc — 直式数学公式编译器 CLI。

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "zsc", version, about = "直式（ZhiShi）数学公式编译器")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 只做语法检查与诊断
    Check {
        /// .zsh 源文件
        file: PathBuf,
        /// JSON 格式诊断
        #[arg(long)]
        json: bool,
    },
    /// 输出 LaTeX 串
    Tex { file: PathBuf },
    /// 打印 AST（调试）
    Ast { file: PathBuf },
    /// 渲染为图片（M4）
    Render {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let path = match &cli.command {
        Commands::Check { file, .. }
        | Commands::Tex { file }
        | Commands::Ast { file }
        | Commands::Render { file, .. } => file.clone(),
    };

    let src = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };

    match &cli.command {
        Commands::Check { json, .. } => match zhishi_core::parse(&src) {
            Err(diag) => {
                if *json {
                    println!(r#"{{"severity":"error","message":{:?}}}"#, diag.message);
                } else {
                    eprintln!("{}", diag.render(&src));
                }
                ExitCode::FAILURE
            }
            Ok(formula) => {
                let result = zhishi_core::checker::check(&formula);
                if *json {
                    for d in &result.diagnostics {
                        println!(
                            r#"{{"severity":"{:?}","message":{:?}}}"#,
                            d.severity, d.message
                        );
                    }
                } else {
                    for d in &result.diagnostics {
                        eprintln!("{}", d.render(&src));
                    }
                }
                if result.has_errors() {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
        },
        Commands::Tex { .. } => match zhishi_core::parse(&src) {
            Err(diag) => {
                eprintln!("{}", diag.render(&src));
                ExitCode::FAILURE
            }
            Ok(formula) => {
                println!("{}", zhishi_tex::to_tex(&formula));
                ExitCode::SUCCESS
            }
        },
        Commands::Ast { .. } => match zhishi_core::parse(&src) {
            Err(diag) => {
                eprintln!("{}", diag.render(&src));
                ExitCode::FAILURE
            }
            Ok(formula) => {
                println!("{formula:#?}");
                ExitCode::SUCCESS
            }
        },
        Commands::Render { output, .. } => {
            let _ = output;
            eprintln!("error: render is not implemented yet (M4)");
            ExitCode::FAILURE
        }
    }
}
