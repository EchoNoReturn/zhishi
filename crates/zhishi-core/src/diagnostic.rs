//! 诊断：解析错误与检查警告的统一表示。

use crate::ast::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    /// 可操作建议，如「建议写 x / (y*z)」
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            help: None,
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            help: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// 人类可读单行/多行输出（M2 起接 ariadne 可替换）。
    pub fn render(&self, src: &str) -> String {
        let sev = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        };
        let mut out = format!("{sev}: {}", self.message);
        if let Some(span) = self.span {
            out.push_str(&format!(
                "\n  --> {}:{}:{}",
                span.line, span.col, span.start
            ));
            if let Some(line) = src.lines().nth(span.line.saturating_sub(1) as usize) {
                out.push_str(&format!("\n   | {line}"));
            }
        }
        if let Some(help) = &self.help {
            out.push_str(&format!("\n  help: {help}"));
        }
        out
    }
}
