//! 语法检查（M2 实现）：歧义警告、名字检查、结构检查。
//! 见规范 §8.2。

use crate::ast::Formula;
use crate::Diagnostic;

pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == crate::Severity::Error)
    }
}

/// 对 AST 做静态检查。M2 前为空实现。
pub fn check(_formula: &Formula) -> CheckResult {
    CheckResult {
        diagnostics: Vec::new(),
    }
}
