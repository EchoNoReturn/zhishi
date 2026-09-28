//! 语法分析（M1 实现）：递归下降，覆盖规范 §3 EBNF 与 §5 优先级。

use crate::ast::Formula;
use crate::Diagnostic;

/// 解析直式源码为公式 AST。
///
/// 优先级总表见规范 §5.5；歧义规则见 §5.1–5.3。
pub fn parse(src: &str) -> Result<Formula, Diagnostic> {
    let _ = src;
    Err(Diagnostic::error("parser: not implemented yet (M1)")
        .with_help("里程碑 M1 将实现规范 §3 的递归下降解析器"))
}
