//! 直式 AST → LaTeX 后端（规范 §8.3 / 例子 §6 右列）。
//!
//! 约定：
//! - 乘法（隐式与 `*`）→ 并列；加减在乘/上下标等紧邻位置自动补括号
//! - `/` → `\frac{}{}`
//! - `'i` → `i`；`f'(x)` 保留素记号
//! - `and`/`or` → `\cap`/`\cup`

use zhishi_core::ast::{BinOp, CaseBranch, Const, Expr, Formula, MatrixKind, UnaryOp};

pub fn to_tex(formula: &Formula) -> String {
    expr_tex(&formula.expr)
}

/// 比对前归一化：折叠空白。
pub fn normalize_tex(tex: &str) -> String {
    tex.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn expr_tex(e: &Expr) -> String {
    match e {
        Expr::Number(n, _) => n.clone(),
        Expr::Var(name, _) => var_tex(name),
        Expr::Greek(name, _) => format!("\\{name}"),
        Expr::Const(c, _) => const_tex(*c),
        Expr::Text(s, _) => format!("\\text{{{s}}}"),
        Expr::Prime { base, order, .. } => {
            format!("{}{}", atom_tex(base), "'".repeat(*order as usize))
        }
        Expr::Call {
            name, args, primes, ..
        } => call_tex(name, args, *primes),
        Expr::Unary { op, expr, .. } => match op {
            UnaryOp::Neg => format!("-{}", atom_tex(expr)),
            UnaryOp::Not => format!("\\neg {}", atom_tex(expr)),
            UnaryOp::Fact => format!("{}!", atom_tex(expr)),
        },
        Expr::Binary { op, lhs, rhs, .. } => binary_tex(*op, lhs, rhs),
        Expr::Sup { base, exp, .. } => format!("{}^{{{}}}", atom_tex(base), expr_tex(exp)),
        Expr::Sub { base, index, .. } => format!("{}_{{{}}}", atom_tex(base), expr_tex(index)),
        Expr::Abs(inner, _) => format!("\\left|{}\\right|", expr_tex(inner)),
        Expr::Norm(inner, _) => format!("\\left\\|{}\\right\\|", expr_tex(inner)),
        Expr::Matrix { kind, rows, .. } => matrix_tex(*kind, rows),
        Expr::Cases { branches, .. } => cases_tex(branches),
        Expr::Align { eqs, .. } => {
            let body: Vec<String> = eqs
                .iter()
                .map(|eq| format!("{} &= {}", expr_tex(&eq.lhs), expr_tex(&eq.rhs)))
                .collect();
            format!(
                "\\begin{{aligned}} {} \\end{{aligned}}",
                body.join(" \\\\ ")
            )
        }
        Expr::Set { elem, pred, .. } => {
            format!("\\{{ {} \\mid {} \\}}", expr_tex(elem), expr_tex(pred))
        }
        Expr::Binom { n, k, .. } => {
            format!("\\binom{{{}}}{{{}}}", expr_tex(n), expr_tex(k))
        }
    }
}

/// 加减等低优先级节点在紧邻上下文（乘、上下标、函数参数外层）补括号。
fn atom_tex(e: &Expr) -> String {
    match e {
        Expr::Binary {
            op:
                BinOp::Add
                | BinOp::Sub
                | BinOp::Pm
                | BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Gt
                | BinOp::Le
                | BinOp::Ge
                | BinOp::Arrow
                | BinOp::And
                | BinOp::Or,
            ..
        } => format!("({})", expr_tex(e)),
        Expr::Unary {
            op: UnaryOp::Neg, ..
        } => format!("({})", expr_tex(e)),
        _ => expr_tex(e),
    }
}

fn const_tex(c: Const) -> String {
    match c {
        Const::Infinity => "\\infty".into(),
        Const::Pi => "\\pi".into(),
        Const::E => "e".into(),
        Const::ImagI => "i".into(),
    }
}

fn var_tex(name: &str) -> String {
    // dx 之类测度：d+单字母 → \, d x（规范 §4.4）
    if name.len() == 2 {
        let mut chars = name.chars();
        if chars.next() == Some('d') {
            if let Some(v) = chars.next() {
                if v.is_ascii_alphabetic() {
                    return format!("\\, d {v}");
                }
            }
        }
    }
    match name {
        "sum" => "\\sum".into(),
        "prod" => "\\prod".into(),
        "int" => "\\int".into(),
        "lim" => "\\lim".into(),
        "inf" => "\\infty".into(),
        "sup" => "\\sup".into(),
        "partial" => "\\partial".into(),
        "oo" => "\\infty".into(),
        "max" => "\\max".into(),
        "min" => "\\min".into(),
        "det" => "\\det".into(),
        _ => name.to_string(),
    }
}

fn binary_tex(op: BinOp, lhs: &Expr, rhs: &Expr) -> String {
    match op {
        // 乘法：并列；加减等操作数补括号；右侧以 ( 开头不插空格
        BinOp::Mul { .. } => {
            let l = atom_tex(lhs);
            let r = atom_tex(rhs);
            if r.starts_with('(') {
                format!("{l}{r}")
            } else {
                format!("{l} {r}")
            }
        }
        BinOp::Div => format!("\\frac{{{}}}{{{}}}", expr_tex(lhs), expr_tex(rhs)),
        _ => {
            let l = expr_tex(lhs);
            let r = expr_tex(rhs);
            let op_s = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Pm => "\\pm",
                BinOp::Eq => "=",
                BinOp::Ne => "\\neq",
                BinOp::Lt => "<",
                BinOp::Gt => ">",
                BinOp::Le => "\\le",
                BinOp::Ge => "\\ge",
                BinOp::Arrow => "\\to",
                BinOp::And => "\\cap",
                BinOp::Or => "\\cup",
                BinOp::Mul { .. } | BinOp::Div => unreachable!(),
            };
            format!("{l} {op_s} {r}")
        }
    }
}

fn call_tex(name: &str, args: &[Expr], primes: u32) -> String {
    let p = "'".repeat(primes as usize);
    let a: Vec<String> = args.iter().map(expr_tex).collect();

    match name {
        "mid" if args.len() == 2 => return format!("{} \\mid {}", a[0], a[1]),
        "angle" if args.len() == 2 => {
            return format!("\\langle {}, {} \\rangle", a[0], a[1]);
        }
        "binom" if args.len() == 2 => {
            return format!("\\binom{{{}}}{{{}}}", a[0], a[1]);
        }
        "root" if args.len() == 2 => return format!("\\sqrt[{}]{{{}}}", a[0], a[1]),
        "sqrt" if args.len() == 1 => return format!("\\sqrt{{{}}}", a[0]),
        "sqrt" if args.len() == 2 => return format!("\\sqrt[{}]{{{}}}", a[1], a[0]),
        "bar" if args.len() == 1 => return format!("\\bar{{{}}}", a[0]),
        "hat" if args.len() == 1 => return format!("\\hat{{{}}}", a[0]),
        "tilde" if args.len() == 1 => return format!("\\tilde{{{}}}", a[0]),
        "dot" if args.len() == 1 => return format!("\\dot{{{}}}", a[0]),
        "ddot" if args.len() == 1 => return format!("\\ddot{{{}}}", a[0]),
        "vec" if args.len() == 1 => return format!("\\vec{{{}}}", a[0]),
        "bold" if args.len() == 1 => return format!("\\boldsymbol{{{}}}", a[0]),
        "overline" if args.len() == 1 => return format!("\\overline{{{}}}", a[0]),
        _ => {}
    }

    let head = match name {
        "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "sinh" | "cosh" | "tanh" | "exp" | "ln"
        | "lg" | "log" | "lim" | "inf" | "sup" | "min" | "max" | "det" | "dim" | "ker" | "gcd"
        | "lcm" | "mod" | "floor" | "ceil" => format!("\\{name}{p}"),
        "arcsin" => format!("\\arcsin{p}"),
        "arccos" => format!("\\arccos{p}"),
        "arctan" => format!("\\arctan{p}"),
        _ => format!("{name}{p}"),
    };

    if a.is_empty() {
        head
    } else {
        format!("{head}({})", a.join(", "))
    }
}

fn matrix_tex(kind: MatrixKind, rows: &[Vec<Expr>]) -> String {
    let env = match kind {
        MatrixKind::Paren => "pmatrix",
        MatrixKind::Bracket => "bmatrix",
        MatrixKind::Vmatrix => "vmatrix",
        MatrixKind::Brace => "matrix",
    };
    let body: Vec<String> = rows
        .iter()
        .map(|row| row.iter().map(expr_tex).collect::<Vec<_>>().join(" & "))
        .collect();
    format!("\\begin{{{env}}} {} \\end{{{env}}}", body.join(" \\\\ "))
}

fn cases_tex(branches: &[CaseBranch]) -> String {
    let body: Vec<String> = branches
        .iter()
        .map(|b| format!("{} & {}", expr_tex(&b.value), expr_tex(&b.cond)))
        .collect();
    format!("\\begin{{cases}} {} \\end{{cases}}", body.join(" \\\\ "))
}
