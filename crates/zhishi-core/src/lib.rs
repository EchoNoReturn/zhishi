//! 直式（ZhiShi）核心：词法、语法、AST、诊断与检查。
//!
//! 规范见 docs/直式语言规范草案.md。AST 是稳定中间表示，
//! 渲染后端（zhishi-tex 等）不得反向影响语法。

pub mod ast;
pub mod checker;
pub mod diagnostic;
pub mod lexer;
pub mod parser;

pub use diagnostic::{Diagnostic, Severity};
pub use parser::parse;
