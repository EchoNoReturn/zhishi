//! 语法分析（规范 §3 EBNF / §5 优先级）：递归下降。

use std::collections::HashSet;

use crate::ast::{
    AlignEq, BinOp, CaseBranch, Const, Expr, FnDecl, Formula, MatrixKind, Span, UnaryOp,
};
use crate::lexer::{const_of, is_function, is_op_name, tokenize, Token, TokenKind};
use crate::Diagnostic;

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    fn_names: HashSet<String>,
    /// |…| / ||…|| 内部禁止把闭定界符当作新原子的开头
    bar_depth: u32,
}

pub fn parse(src: &str) -> Result<Formula, Diagnostic> {
    let tokens = tokenize(src)?;
    let mut p = Parser {
        tokens,
        pos: 0,
        fn_names: HashSet::new(),
        bar_depth: 0,
    };
    p.parse_formula()
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_at(&self, n: usize) -> &TokenKind {
        let i = (self.pos + n).min(self.tokens.len() - 1);
        &self.tokens[i].kind
    }

    fn span(&self) -> Span {
        self.peek().span
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos.min(self.tokens.len() - 1)].clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn err_help(&self, msg: impl Into<String>, help: impl Into<String>) -> Diagnostic {
        Diagnostic::error(msg)
            .with_span(self.span())
            .with_help(help)
    }

    fn eat_op(&mut self, op: &str) -> bool {
        if matches!(self.peek_kind(), TokenKind::Op(s) if s == op) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_op(&mut self, op: &str) -> Result<Span, Diagnostic> {
        if self.eat_op(op) {
            Ok(self.tokens[self.pos.saturating_sub(1)].span)
        } else {
            Err(self.err_help(
                format!("expected `{op}`"),
                format!("here we need `{op}` to continue"),
            ))
        }
    }

    fn at_op(&self, op: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Op(s) if s == op)
    }

    fn at_ident(&self, name: &str) -> bool {
        matches!(self.peek_kind(), TokenKind::Ident(s) if s == name)
    }

    // ── formula ──────────────────────────────────────────────

    fn parse_formula(&mut self) -> Result<Formula, Diagnostic> {
        let mut fn_decls = Vec::new();
        while matches!(self.peek_kind(), TokenKind::Fn) {
            fn_decls.push(self.parse_fn_decl()?);
        }
        let expr = self.parse_expr()?;
        if !matches!(self.peek_kind(), TokenKind::Eof) {
            return Err(self.err_help(
                format!("unexpected trailing input: {:?}", self.peek_kind()),
                "a formula is a single expression; wrap extra parts in parens",
            ));
        }
        Ok(Formula { expr, fn_decls })
    }

    fn parse_fn_decl(&mut self) -> Result<FnDecl, Diagnostic> {
        let start = self.bump().span; // fn
        let name_tok = self.bump();
        let name = match name_tok.kind {
            TokenKind::Ident(s) => s,
            _ => {
                return Err(
                    Diagnostic::error("expected function name after `fn`").with_span(name_tok.span)
                )
            }
        };
        if self.fn_names.contains(&name) || is_function(&name) {
            return Err(
                Diagnostic::error(format!("function `{name}` already defined"))
                    .with_span(name_tok.span),
            );
        }
        self.expect_op("(")?;
        let mut params = Vec::new();
        if !self.at_op(")") {
            loop {
                let t = self.bump();
                match t.kind {
                    TokenKind::Ident(s) => {
                        if params.contains(&s) {
                            return Err(Diagnostic::error(format!("duplicate parameter `{s}`"))
                                .with_span(t.span));
                        }
                        params.push(s);
                    }
                    _ => return Err(Diagnostic::error("expected parameter name").with_span(t.span)),
                }
                if !self.eat_op(",") {
                    break;
                }
            }
        }
        self.expect_op(")")?;
        self.fn_names.insert(name.clone());
        let body = if self.eat_op("=") {
            Some(self.parse_expr()?)
        } else {
            None
        };
        Ok(FnDecl {
            name,
            params,
            body,
            span: start,
        })
    }

    // ── expression levels (规范 §5.5) ───────────────────────

    /// rel = sum { relop sum }
    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_sum()?;
        while let Some(op) = self.rel_op() {
            self.bump();
            let rhs = self.parse_sum()?;
            let span = self.join_span(&lhs, &rhs);
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn rel_op(&self) -> Option<BinOp> {
        match self.peek_kind() {
            TokenKind::Op(s) => match s.as_str() {
                "=" | "==" => Some(BinOp::Eq),
                "!=" => Some(BinOp::Ne),
                "<" => Some(BinOp::Lt),
                ">" => Some(BinOp::Gt),
                "<=" => Some(BinOp::Le),
                ">=" => Some(BinOp::Ge),
                "->" => Some(BinOp::Arrow),
                _ => None,
            },
            TokenKind::Ident(s) if s == "and" => Some(BinOp::And),
            TokenKind::Ident(s) if s == "or" => Some(BinOp::Or),
            _ => None,
        }
    }

    /// sum = term { ("+"|"-"|"pm") term }
    fn parse_sum(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_term()?;
        loop {
            let op = match self.peek_kind() {
                TokenKind::Op(s) => match s.as_str() {
                    "+" => Some(BinOp::Add),
                    "-" => Some(BinOp::Sub),
                    "pm" => Some(BinOp::Pm),
                    _ => None,
                },
                _ => None,
            };
            let Some(op) = op else { break };
            self.bump();
            let rhs = self.parse_term()?;
            let span = self.join_span(&lhs, &rhs);
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    /// term = unary { ("*"|"/"|juxtapose) unary }  — 左结合（规范 §5.2）
    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut lhs = self.parse_unary()?;
        loop {
            let (op, explicit) = if self.at_op("*") {
                self.bump();
                (BinOp::Mul { implicit: false }, true)
            } else if self.at_op("/") {
                self.bump();
                (BinOp::Div, true)
            } else if self.starts_atom() {
                (BinOp::Mul { implicit: true }, false)
            } else {
                break;
            };
            let rhs = self.parse_unary()?;
            let span = self.join_span(&lhs, &rhs);
            let _ = explicit;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    /// unary = "-" unary | postfix
    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.at_op("-") {
            let start = self.bump().span;
            let expr = self.parse_unary()?;
            let span = Span::new(start.start, self.prev_end(), start.line, start.col);
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
                span,
            });
        }
        if self.at_ident("not") {
            let start = self.bump().span;
            let expr = self.parse_unary()?;
            let span = Span::new(start.start, self.prev_end(), start.line, start.col);
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                span,
            });
        }
        self.parse_scripted()
    }

    /// scripted = atom-primes { "^" atom | "_" atom | "!" }
    /// `^` 右结合（指数本身可再带 ^）；`_`/`!` 左结合。
    fn parse_scripted(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.parse_atom()?;
        loop {
            if self.at_op("^") {
                self.bump();
                let exp = self.parse_scripted()?; // 右结合
                let span = self.join_span(&e, &exp);
                e = Expr::Sup {
                    base: Box::new(e),
                    exp: Box::new(exp),
                    span,
                };
            } else if self.at_op("_") {
                self.bump();
                let index = self.parse_atom()?;
                let span = self.join_span(&e, &index);
                e = Expr::Sub {
                    base: Box::new(e),
                    index: Box::new(index),
                    span,
                };
            } else if self.at_op("!") {
                let t = self.bump();
                let span = Span::new(
                    self.span_of(&e).start,
                    t.span.end,
                    self.span_of(&e).line,
                    self.span_of(&e).col,
                );
                e = Expr::Unary {
                    op: UnaryOp::Fact,
                    expr: Box::new(e),
                    span,
                };
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn starts_atom(&self) -> bool {
        match self.peek_kind() {
            TokenKind::Number(_) | TokenKind::Greek(_) | TokenKind::ImagI | TokenKind::Text(_) => {
                true
            }
            TokenKind::Ident(s) => {
                // 逻辑/分隔关键字不是原子
                if matches!(s.as_str(), "and" | "or" | "not" | "otherwise") {
                    return false;
                }
                // bare `mid` 是集合分隔符
                if s == "mid" && !matches!(self.peek_at(1), TokenKind::Op(o) if o == "(") {
                    return false;
                }
                true
            }
            TokenKind::Op(s) => match s.as_str() {
                "(" | "{" => true,
                // | / || 在 bar 上下文里是闭定界符，不能当作新原子
                "|" | "||" => self.bar_depth == 0,
                _ => false,
            },
            _ => false,
        }
    }

    fn span_of(&self, e: &Expr) -> Span {
        match e {
            Expr::Number(_, s)
            | Expr::Var(_, s)
            | Expr::Greek(_, s)
            | Expr::Const(_, s)
            | Expr::Prime { span: s, .. }
            | Expr::Call { span: s, .. }
            | Expr::Unary { span: s, .. }
            | Expr::Binary { span: s, .. }
            | Expr::Sup { span: s, .. }
            | Expr::Sub { span: s, .. }
            | Expr::Abs(_, s)
            | Expr::Norm(_, s)
            | Expr::Matrix { span: s, .. }
            | Expr::Cases { span: s, .. }
            | Expr::Align { span: s, .. }
            | Expr::Set { span: s, .. }
            | Expr::Text(_, s)
            | Expr::Binom { span: s, .. } => *s,
        }
    }

    fn prev_end(&self) -> usize {
        if self.pos == 0 {
            return 0;
        }
        self.tokens[self.pos - 1].span.end
    }

    fn join_span(&self, a: &Expr, b: &Expr) -> Span {
        let sa = self.span_of(a);
        let sb = self.span_of(b);
        Span::new(sa.start, sb.end, sa.line, sa.col)
    }

    // ── atoms ───────────────────────────────────────────────

    fn parse_atom(&mut self) -> Result<Expr, Diagnostic> {
        match self.peek_kind().clone() {
            TokenKind::Number(n) => {
                let t = self.bump();
                Ok(Expr::Number(n, t.span))
            }
            TokenKind::Text(s) => {
                let t = self.bump();
                Ok(Expr::Text(s, t.span))
            }
            TokenKind::ImagI => {
                let t = self.bump();
                Ok(Expr::Const(Const::ImagI, t.span))
            }
            TokenKind::Greek(name) => {
                let t = self.bump();
                Ok(Expr::Greek(name, t.span))
            }
            TokenKind::Ident(name) => self.parse_ident_atom(name),
            TokenKind::Op(s) if s == "(" => {
                let start = self.bump().span;
                let e = self.parse_expr()?;
                self.expect_op(")")?;
                let _ = start;
                Ok(e)
            }
            TokenKind::Op(s) if s == "|" => self.parse_abs_or_norm(),
            TokenKind::Op(s) if s == "||" => self.parse_norm(),
            TokenKind::Op(s) if s == "{" => self.parse_set(),
            other => Err(self.err_help(
                format!("expected expression, found {other:?}"),
                "start a formula with a number, name, `(`, `|`, or `{`",
            )),
        }
    }

    fn parse_ident_atom(&mut self, name: String) -> Result<Expr, Diagnostic> {
        let name_tok = self.bump(); // 消费 Ident

        // 结构块：matrix/cases/align
        if matches!(name.as_str(), "matrix" | "bmatrix" | "vmatrix" | "pmatrix")
            && (self.at_op("(") || self.at_op("[") || self.at_op("{"))
        {
            return self.parse_matrix(&name);
        }
        if name == "cases" && self.at_op("{") {
            return self.parse_cases();
        }
        if name == "align" && self.at_op("{") {
            return self.parse_align();
        }

        // name + primes + (args) → call（含 f'(x)）
        let mut primes = 0u32;
        while matches!(self.peek_kind(), TokenKind::Prime) {
            self.bump();
            primes += 1;
        }

        let is_call =
            self.at_op("(") && (primes > 0 || is_function(&name) || self.fn_names.contains(&name));

        if is_call {
            let name_span = name_tok.span;
            self.bump(); // (
            let mut args = Vec::new();
            if !self.at_op(")") {
                loop {
                    args.push(self.parse_expr()?);
                    if !self.eat_op(",") {
                        break;
                    }
                }
            }
            let end = self.expect_op(")")?;
            return Ok(Expr::Call {
                name,
                args,
                primes,
                span: Span::new(name_span.start, end.end, name_span.line, name_span.col),
            });
        }

        let t_span = name_tok.span;
        let mut e = if let Some(c) = const_of(&name) {
            Expr::Const(c, t_span)
        } else if name == "otherwise" {
            Expr::Text("otherwise".into(), t_span)
        } else {
            Expr::Var(name, t_span)
        };

        if primes > 0 {
            let end = self.tokens[self.pos.saturating_sub(1)].span.end;
            let span = Span::new(t_span.start, end, t_span.line, t_span.col);
            e = Expr::Prime {
                base: Box::new(e),
                order: primes,
                span,
            };
        }
        Ok(e)
    }

    fn parse_abs_or_norm(&mut self) -> Result<Expr, Diagnostic> {
        self.expect_op("|")?;
        self.bar_depth += 1;
        let e = self.parse_expr()?;
        self.bar_depth -= 1;
        self.expect_op("|")?;
        Ok(Expr::Abs(Box::new(e), self.span()))
    }

    fn parse_norm(&mut self) -> Result<Expr, Diagnostic> {
        self.expect_op("||")?;
        self.bar_depth += 1;
        let e = self.parse_expr()?;
        self.bar_depth -= 1;
        self.expect_op("||")?;
        Ok(Expr::Norm(Box::new(e), self.span()))
    }

    fn parse_set(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.expect_op("{")?;
        let elem = self.parse_expr()?;
        if !self.at_ident("mid") {
            return Err(self.err_help(
                "set builder requires `mid` separator",
                "write `{x mid x > 0}`",
            ));
        }
        self.bump(); // mid
        let pred = self.parse_expr()?;
        let end = self.expect_op("}")?;
        Ok(Expr::Set {
            elem: Box::new(elem),
            pred: Box::new(pred),
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_matrix(&mut self, name: &str) -> Result<Expr, Diagnostic> {
        let start = self.span();
        let open = if self.at_op("(") {
            "("
        } else if self.at_op("[") {
            "["
        } else {
            "{"
        };
        let close = match open {
            "(" => ")",
            "[" => "]",
            _ => "}",
        };
        let kind = match name {
            "vmatrix" => MatrixKind::Vmatrix,
            "bmatrix" => MatrixKind::Bracket,
            "pmatrix" => MatrixKind::Paren,
            _ => match open {
                "[" => MatrixKind::Bracket,
                "{" => MatrixKind::Brace,
                _ => MatrixKind::Paren,
            },
        };
        self.expect_op(open)?;
        let mut rows: Vec<Vec<Expr>> = Vec::new();
        let mut cur: Vec<Expr> = Vec::new();
        if !self.at_op(close) {
            loop {
                if self.at_op(";") {
                    rows.push(std::mem::take(&mut cur));
                    self.bump();
                    continue;
                }
                if self.at_op(close) {
                    break;
                }
                cur.push(self.parse_expr()?);
                if self.eat_op(",") {
                    continue;
                }
                if self.at_op(";") || self.at_op(close) {
                    continue;
                }
                return Err(self.err_help(
                    "expected `,` or `;` between matrix entries",
                    "rows are split by `;`, columns by `,`",
                ));
            }
        }
        rows.push(cur);
        // 去掉尾部空行（`3, 4;` 这类）
        if rows.len() > 1 && rows.last().map(|r| r.is_empty()).unwrap_or(false) {
            rows.pop();
        }
        let end = self.expect_op(close)?;
        Ok(Expr::Matrix {
            kind,
            rows,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_cases(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.span();
        self.expect_op("{")?;
        let mut branches = Vec::new();
        loop {
            if self.at_op("}") {
                break;
            }
            let value = self.parse_expr()?;
            self.expect_op(",")?;
            let cond = if self.at_ident("otherwise") {
                let t = self.bump();
                Expr::Text("otherwise".into(), t.span)
            } else {
                self.parse_expr()?
            };
            branches.push(CaseBranch { value, cond });
            if self.eat_op(";") {
                continue;
            }
            break;
        }
        let end = self.expect_op("}")?;
        if branches.is_empty() {
            return Err(Diagnostic::error("cases must have at least one branch")
                .with_span(Span::new(start.start, end.end, start.line, start.col)));
        }
        Ok(Expr::Cases {
            branches,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_align(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.span();
        self.expect_op("{")?;
        let mut eqs = Vec::new();
        loop {
            if self.at_op("}") {
                break;
            }
            let lhs = self.parse_sum()?;
            self.expect_op("&=")?;
            let rhs = self.parse_expr()?;
            eqs.push(AlignEq { lhs, rhs });
            if self.eat_op(";") {
                continue;
            }
            break;
        }
        let end = self.expect_op("}")?;
        if eqs.is_empty() {
            return Err(Diagnostic::error("align must have at least one equation")
                .with_span(Span::new(start.start, end.end, start.line, start.col)));
        }
        Ok(Expr::Align {
            eqs,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }
}

/// 供 checker / tex 使用：名字是否为已知函数。
pub fn is_known_function(name: &str) -> bool {
    is_function(name) || is_op_name(name)
}

/// 仅供测试：快速断言可解析。
#[cfg(test)]
mod tests {
    use super::*;

    fn ok(src: &str) {
        parse(src).unwrap_or_else(|e| panic!("parse failed for {src:?}: {}", e.message));
    }

    #[test]
    fn basic_expr() {
        ok("a + b");
        ok("a/b");
        ok("(-b ± sqrt(b^2 - 4*a*c)) / (2*a)");
        ok("x^2");
        ok("x_i^2");
        ok("x^(i+1)");
        ok("x'");
        ok("f'(x)");
        ok("y''");
    }

    #[test]
    fn scripts_and_ops() {
        ok("sum_(i=1)^n i^2");
        ok("lim_(x->0) sin(x)/x");
        ok("int_0^oo exp(-x^2) dx");
        ok("prod_(k=1)^n k = n!");
    }

    #[test]
    fn structures() {
        ok("cases{ x^2, x > 0; 0, otherwise }");
        ok("matrix(1, 2; 3, 4)");
        ok("vmatrix(a, b; c, d)");
        ok("align{ 2*x + y &= 5; x - y &= 1 }");
        ok("{x mid x > 0}");
    }

    #[test]
    fn calls_and_fn() {
        ok("sin(x)");
        ok("exp('i * pi) + 1 = 0");
        ok("fn P(x) P(mid(A, B)) = P(A and B) / P(B)");
        ok("binom(n, k)");
        ok("bar(x) = (1/n) sum_(i=1)^n x_i");
    }

    #[test]
    fn juxtaposition() {
        ok("2x");
        ok("3(x+1)");
        ok("(a+b)(a-b)");
        ok("n (n+1) (2 n+1)");
    }

    #[test]
    fn primes_and_imag() {
        ok("x' * 'i");
        ok("exp('i * pi)");
    }
}
