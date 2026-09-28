//! 词法分析（规范 §2）。
//!
//! 覆盖：数字、标识符、保留字、`'i`、素记号 `'`、运算符、字符串、Unicode 别名归一。

use crate::ast::{Const, Span};
use crate::Diagnostic;

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
    /// 运算符/定界符，如 + - * / ^ _ | || ( ) [ ] { } , ; =
    Op(String),
    /// 函数声明关键字 fn
    Fn,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// 希腊字母保留字（规范 §2.3）。
pub const GREEK: &[&str] = &[
    "alpha",
    "beta",
    "gamma",
    "delta",
    "epsilon",
    "varepsilon",
    "zeta",
    "eta",
    "theta",
    "vartheta",
    "iota",
    "kappa",
    "lambda",
    "mu",
    "nu",
    "xi",
    "pi",
    "rho",
    "sigma",
    "tau",
    "upsilon",
    "phi",
    "varphi",
    "chi",
    "psi",
    "omega",
    "Gamma",
    "Delta",
    "Theta",
    "Lambda",
    "Xi",
    "Pi",
    "Sigma",
    "Upsilon",
    "Phi",
    "Psi",
    "Omega",
];

/// 调用形式的函数名（规范 §2.3）。
pub const FUNCTIONS: &[&str] = &[
    "sin", "cos", "tan", "cot", "sec", "csc", "arcsin", "arccos", "arctan", "sinh", "cosh", "tanh",
    "exp", "ln", "lg", "log", "lim", "inf", "sup", "min", "max", "det", "dim", "ker", "gcd", "lcm",
    "mod", "floor", "ceil", "mid", "binom", "root", "bar", "hat", "tilde", "dot", "ddot", "vec",
    "bold", "overline", "angle", "sqrt",
];

/// 可带上下限的大算符/算子名（sum_(i=1)^n …）。
pub const OP_NAMES: &[&str] = &[
    "sum", "prod", "int", "lim", "inf", "sup", "partial", "max", "min", "det", "gcd", "lcm", "Pr",
];

/// 逻辑关键字。
pub const LOGIC: &[&str] = &["and", "or", "not"];

/// 结构关键字。
pub const KEYWORDS: &[&str] = &[
    "fn",
    "cases",
    "align",
    "matrix",
    "bmatrix",
    "vmatrix",
    "pmatrix",
    "otherwise",
];

pub fn is_greek(name: &str) -> bool {
    GREEK.contains(&name)
}

pub fn is_function(name: &str) -> bool {
    FUNCTIONS.contains(&name)
}

pub fn is_op_name(name: &str) -> bool {
    OP_NAMES.contains(&name)
}

pub fn is_logic(name: &str) -> bool {
    LOGIC.contains(&name)
}

pub fn is_keyword(name: &str) -> bool {
    KEYWORDS.contains(&name)
}

pub fn const_of(name: &str) -> Option<Const> {
    match name {
        "oo" => Some(Const::Infinity),
        "e" => Some(Const::E),
        _ => None,
    }
}

/// pi 是希腊字母，不在 const_of 中（走 Greek 通道）。
pub fn is_pseudo_const(name: &str) -> bool {
    matches!(name, "oo" | "e")
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic()
}

fn is_ident_cont(c: char) -> bool {
    // `_` 不进标识符：x_i 是下标，f_max 是 f_{max}（规范 §2.1）
    c.is_ascii_alphanumeric()
}

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut it = self.src[self.pos..].chars();
        it.next();
        it.next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn span_from(&self, start: usize, start_line: u32, start_col: u32) -> Span {
        Span::new(start, self.pos, start_line, start_col)
    }

    fn err(&self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::error(msg).with_span(Span::new(self.pos, self.pos, self.line, self.col))
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        loop {
            self.skip_ws();
            let start = self.pos;
            let start_line = self.line;
            let start_col = self.col;
            let Some(c) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: self.span_from(start, start_line, start_col),
                });
                return Ok(tokens);
            };

            let kind = if c.is_ascii_digit() {
                self.lex_number()?
            } else if is_ident_start(c) {
                self.lex_ident()
            } else if c == '"' {
                self.lex_text()?
            } else if c == '\'' {
                self.bump();
                // 'i 虚数单位；否则素记号
                if self.peek() == Some('i')
                    && self.peek2().map(|n| !is_ident_cont(n)).unwrap_or(true)
                {
                    self.bump();
                    TokenKind::ImagI
                } else {
                    TokenKind::Prime
                }
            } else {
                self.lex_op()?
            };

            tokens.push(Token {
                kind,
                span: self.span_from(start, start_line, start_col),
            });
        }
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else if c == '%' {
                // TeX 风格行注释
                while let Some(c) = self.bump() {
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn lex_number(&mut self) -> Result<TokenKind, Diagnostic> {
        let mut s = String::new();
        // 整数/小数
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                s.push(c);
                self.bump();
            } else if (c == 'e' || c == 'E')
                && self
                    .peek2()
                    .map(|n| n.is_ascii_digit() || n == '+' || n == '-')
                    .unwrap_or(false)
            {
                s.push(c);
                self.bump();
                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        s.push(sign);
                        self.bump();
                    }
                }
                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        s.push(c);
                        self.bump();
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }
        if s.is_empty() {
            return Err(self.err("invalid number"));
        }
        Ok(TokenKind::Number(s))
    }

    fn lex_ident(&mut self) -> TokenKind {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if is_ident_cont(c) {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        // 完整标识符才可能是希腊/常量/关键字；omegax 是普通变量
        if is_greek(&s) {
            TokenKind::Greek(s)
        } else if s == "fn" {
            TokenKind::Fn
        } else {
            TokenKind::Ident(s)
        }
    }

    fn lex_text(&mut self) -> Result<TokenKind, Diagnostic> {
        self.bump(); // "
        let mut s = String::new();
        loop {
            match self.bump() {
                Some('"') => return Ok(TokenKind::Text(s)),
                Some('\\') => match self.bump() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some(c) => s.push(c),
                    None => return Err(self.err("unterminated string escape")),
                },
                Some(c) => s.push(c),
                None => return Err(self.err("unterminated string")),
            }
        }
    }

    fn lex_op(&mut self) -> Result<TokenKind, Diagnostic> {
        let c = self.peek().unwrap();
        // 双字符优先
        let two: String = [self.peek(), self.peek2()]
            .into_iter()
            .flatten()
            .collect::<String>()
            .chars()
            .take(2)
            .collect();
        let two: String = two.chars().take(2).collect();

        let op = match two.as_str() {
            "||" => {
                self.bump();
                self.bump();
                "||"
            }
            "<=" => {
                self.bump();
                self.bump();
                "<="
            }
            ">=" => {
                self.bump();
                self.bump();
                ">="
            }
            "!=" => {
                self.bump();
                self.bump();
                "!="
            }
            "->" => {
                self.bump();
                self.bump();
                "->"
            }
            "&=" => {
                self.bump();
                self.bump();
                "&="
            }
            _ => {
                // Unicode 别名归一（规范 §2.4）
                match c {
                    '±' => {
                        self.bump();
                        "pm"
                    }
                    '∞' => {
                        self.bump();
                        return Ok(TokenKind::Ident("oo".into()));
                    }
                    '≤' => {
                        self.bump();
                        "<="
                    }
                    '≥' => {
                        self.bump();
                        ">="
                    }
                    '≠' => {
                        self.bump();
                        "!="
                    }
                    '→' => {
                        self.bump();
                        "->"
                    }
                    '·' | '×' => {
                        self.bump();
                        "*"
                    }
                    '∑' => {
                        self.bump();
                        return Ok(TokenKind::Ident("sum".into()));
                    }
                    '∏' => {
                        self.bump();
                        return Ok(TokenKind::Ident("prod".into()));
                    }
                    '∫' => {
                        self.bump();
                        return Ok(TokenKind::Ident("int".into()));
                    }
                    '√' => {
                        self.bump();
                        return Ok(TokenKind::Ident("sqrt".into()));
                    }
                    'π' => {
                        self.bump();
                        return Ok(TokenKind::Greek("pi".into()));
                    }
                    '+' | '-' | '*' | '/' | '^' | '_' | '|' | '(' | ')' | '[' | ']' | '{' | '}'
                    | ',' | ';' | '=' | '<' | '>' | '!' | '&' => {
                        self.bump();
                        // 单字符 op，存字符串
                        let mut buf = [0u8; 4];
                        let s = c.encode_utf8(&mut buf);
                        return Ok(TokenKind::Op(s.to_string()));
                    }
                    other => {
                        return Err(self.err(format!("unexpected character {other:?}")));
                    }
                }
            }
        };
        Ok(TokenKind::Op(op.to_string()))
    }
}

/// 将源码切分为 token 流（末尾为 Eof）。
pub fn tokenize(src: &str) -> Result<Vec<Token>, Diagnostic> {
    Lexer::new(src).tokenize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn numbers_and_idents() {
        assert_eq!(
            kinds("1 + x2"),
            vec![
                TokenKind::Number("1".into()),
                TokenKind::Op("+".into()),
                TokenKind::Ident("x2".into()),
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn imag_i_and_prime() {
        assert_eq!(kinds("'i"), vec![TokenKind::ImagI, TokenKind::Eof]);
        assert_eq!(
            kinds("y'"),
            vec![
                TokenKind::Ident("y".into()),
                TokenKind::Prime,
                TokenKind::Eof
            ]
        );
        assert_eq!(
            kinds("x' * 'i"),
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::Prime,
                TokenKind::Op("*".into()),
                TokenKind::ImagI,
                TokenKind::Eof
            ]
        );
    }

    #[test]
    fn greek_is_whole_ident_only() {
        assert_eq!(
            kinds("alpha"),
            vec![TokenKind::Greek("alpha".into()), TokenKind::Eof]
        );
        assert_eq!(
            kinds("omegax"),
            vec![TokenKind::Ident("omegax".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn unicode_aliases() {
        assert_eq!(kinds("±"), vec![TokenKind::Op("pm".into()), TokenKind::Eof]);
        assert_eq!(
            kinds("oo"),
            vec![TokenKind::Ident("oo".into()), TokenKind::Eof]
        );
    }

    #[test]
    fn double_char_ops() {
        assert_eq!(
            kinds("x <= y -> z"),
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::Op("<=".into()),
                TokenKind::Ident("y".into()),
                TokenKind::Op("->".into()),
                TokenKind::Ident("z".into()),
                TokenKind::Eof
            ]
        );
    }
}
