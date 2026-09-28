//! 词法分析（M1 实现）。
//!
//! 需覆盖：数字、标识符、保留字（希腊/函数/常量）、'i、素记号 '、
//! 运算符、字符串、Unicode 别名归一（规范 §2）。

use crate::ast::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(String),
    Ident(String),
    /// 希腊保留字，存规范名
    Greek(String),
    /// 虚数单位 'i
    ImagI,
    /// 素记号 '
    Prime,
    /// "..." 文本
    Text(String),
    Op(String),
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// M1 起实现；当前返回未实现错误，保持 API 稳定。
pub fn tokenize(_src: &str) -> Result<Vec<Token>, crate::Diagnostic> {
    Err(crate::Diagnostic::error("lexer: not implemented yet (M1)"))
}
