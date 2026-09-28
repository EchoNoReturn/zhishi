//! 直式 AST → LaTeX 后端（M3 实现）。
//!
//! 黄金测试：examples/*.zsh → 同名 *.tex，文本比对（空白/{} 归一化后相等）。

use zhishi_core::ast::Formula;

/// 将公式 AST 翻译为 LaTeX 数学串（不含 `$`/`\[` 包裹）。
pub fn to_tex(_formula: &Formula) -> String {
    // M3 实现；当前返回空串，让黄金测试红着（M0 验收）。
    String::new()
}

/// 比对前归一化：折叠空白、去掉冗余 `{}` 差异的最小规则。
pub fn normalize_tex(tex: &str) -> String {
    tex.split_whitespace().collect::<Vec<_>>().join(" ")
}
