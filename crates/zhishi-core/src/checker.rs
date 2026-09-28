//! 语法检查（M2）：歧义警告、名字检查、结构检查。规范 §8.2。

use crate::ast::{BinOp, Expr, Formula, UnaryOp};
use crate::lexer::{is_function, is_greek, is_keyword, is_op_name};
use crate::{Diagnostic, Severity};

pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

/// 对 AST 做静态检查。
pub fn check(formula: &Formula) -> CheckResult {
    let mut ctx = Ctx {
        diagnostics: Vec::new(),
        fn_names: formula.fn_decls.iter().map(|d| d.name.clone()).collect(),
    };

    for decl in &formula.fn_decls {
        for p in &decl.params {
            if is_greek(p) || is_keyword(p) || is_function(p) || is_op_name(p) {
                ctx.warn(
                    format!("parameter `{p}` shadows a reserved word"),
                    Some(decl.span),
                    "rename the parameter to avoid confusion",
                );
            }
        }
    }

    ctx.walk(&formula.expr);
    CheckResult {
        diagnostics: ctx.diagnostics,
    }
}

struct Ctx {
    diagnostics: Vec<Diagnostic>,
    fn_names: Vec<String>,
}

impl Ctx {
    fn warn(&mut self, msg: impl Into<String>, span: Option<crate::ast::Span>, help: &str) {
        let mut d = Diagnostic {
            severity: Severity::Warning,
            message: msg.into(),
            span,
            help: Some(help.into()),
        };
        if span.is_none() {
            d.span = None;
        }
        self.diagnostics.push(d);
    }

    fn walk(&mut self, e: &Expr) {
        match e {
            Expr::Binary {
                op: BinOp::Div,
                lhs,
                rhs,
                ..
            } => {
                self.walk(lhs);
                self.walk(rhs);
            }
            Expr::Binary {
                op: BinOp::Mul { implicit: true },
                lhs,
                rhs,
                span,
            } => {
                // §5.2：x / y z 解析成 (x/y)*z —— 分母后紧跟并列
                if matches!(&**lhs, Expr::Binary { op: BinOp::Div, .. }) {
                    self.warn(
                        "division followed by juxtaposition may be misread",
                        Some(*span),
                        "write `x / (y * z)` if the product is the denominator",
                    );
                }
                // §5.1：a(b) 默认乘法，但 a 像函数名
                if let Expr::Var(name, _) = &**lhs {
                    if looks_like_fn(name) && !self.fn_names.iter().any(|f| f == name) {
                        self.warn(
                            format!("`{name}(...)` was parsed as multiplication"),
                            Some(*span),
                            &format!("declare `fn {name}(...)` if this is a function call"),
                        );
                    }
                }
                self.walk(lhs);
                self.walk(rhs);
            }
            Expr::Binary { lhs, rhs, .. } => {
                self.walk(lhs);
                self.walk(rhs);
            }
            Expr::Unary { expr, op, .. } => {
                if matches!(op, UnaryOp::Neg | UnaryOp::Not | UnaryOp::Fact) {
                    // Fact 无子检查
                }
                self.walk(expr);
            }
            Expr::Sup { base, exp, .. }
            | Expr::Sub {
                base, index: exp, ..
            } => {
                self.walk(base);
                self.walk(exp);
            }
            Expr::Prime { base, .. } => self.walk(base),
            Expr::Call {
                name, args, span, ..
            } => {
                if !is_function(name)
                    && !is_op_name(name)
                    && !self.fn_names.iter().any(|f| f == name)
                {
                    self.warn(
                        format!("call to undeclared function `{name}`"),
                        Some(*span),
                        &format!("add `fn {name}(...)` at the top of the file"),
                    );
                }
                for a in args {
                    self.walk(a);
                }
            }
            Expr::Abs(inner, _) | Expr::Norm(inner, _) => self.walk(inner),
            Expr::Matrix { rows, span, .. } => {
                let width = rows.first().map(|r| r.len()).unwrap_or(0);
                for row in rows {
                    if row.len() != width {
                        self.warn(
                            "matrix rows have different lengths",
                            Some(*span),
                            "pad short rows with explicit zeros if intended",
                        );
                        break;
                    }
                }
                for row in rows {
                    for cell in row {
                        self.walk(cell);
                    }
                }
            }
            Expr::Cases { branches, span, .. } => {
                let has_otherwise = branches
                    .iter()
                    .any(|b| matches!(&b.cond, Expr::Text(s, _) if s == "otherwise"));
                if !has_otherwise {
                    self.warn(
                        "cases has no `otherwise` branch",
                        Some(*span),
                        "add a default branch `; expr, otherwise` if the list is exhaustive",
                    );
                }
                for b in branches {
                    self.walk(&b.value);
                    self.walk(&b.cond);
                }
            }
            Expr::Align { eqs, .. } => {
                for eq in eqs {
                    self.walk(&eq.lhs);
                    self.walk(&eq.rhs);
                }
            }
            Expr::Set { elem, pred, .. } => {
                self.walk(elem);
                self.walk(pred);
            }
            Expr::Binom { n, k, .. } => {
                self.walk(n);
                self.walk(k);
            }
            Expr::Number(_, _)
            | Expr::Var(_, _)
            | Expr::Greek(_, _)
            | Expr::Const(_, _)
            | Expr::Text(_, _) => {}
        }
    }
}

fn looks_like_fn(name: &str) -> bool {
    // 形如 f/g/P/Foo，后面又跟了括号乘法 —— 提示可能本意是调用
    name.chars().all(|c| c.is_ascii_alphabetic()) && name.len() <= 3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn warn_div_juxtaposition() {
        let f = parse("x / y z").unwrap();
        let r = check(&f);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.message.contains("juxtaposition")));
    }

    #[test]
    fn warn_mul_looking_like_call() {
        let f = parse("fn f(x) f(x) + h(1)").unwrap();
        let r = check(&f);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.message.contains("`h(...)` was parsed as multiplication")));
    }

    #[test]
    fn warn_cases_otherwise() {
        let f = parse("cases{ x, x > 0; 0, x <= 0 }").unwrap();
        let r = check(&f);
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d.message.contains("otherwise")));
    }

    #[test]
    fn warn_matrix_ragged() {
        let f = parse("matrix(1, 2; 3)").unwrap();
        let r = check(&f);
        assert!(r.diagnostics.iter().any(|d| d.message.contains("rows")));
    }

    #[test]
    fn ok_quiet_on_clean_formula() {
        let f = parse("sum_(i=1)^n i^2 = n*(n+1)/6").unwrap();
        let r = check(&f);
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    }
}
