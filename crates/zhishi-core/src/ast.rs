//! 直式 AST。节点集合与规范 §3 构造一一对应。

use std::fmt;

/// 源码位置（1-based 行列，字节偏移）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub col: u32,
}

impl Span {
    pub fn new(start: usize, end: usize, line: u32, col: u32) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }
}

/// 内置常量。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Const {
    /// oo → ∞
    Infinity,
    /// pi → π
    Pi,
    /// e → 自然常数
    E,
    /// 'i → 虚数单位
    ImagI,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    /// →
    Arrow,
    /// and
    And,
    /// or
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

/// 矩阵定界符（matrix / bmatrix / vmatrix / pmatrix + 括号形态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixKind {
    /// matrix(...) 圆括号
    Paren,
    /// matrix[...] 方括号
    Bracket,
    /// matrix{...} 无界
    Brace,
    /// bmatrix 等显式前缀（保留）
    Named(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// 数字字面量（保留原文，避免过早丢精度）
    Number(String, Span),
    /// 变量/未保留标识符
    Var(String, Span),
    /// 希腊字母保留字（存规范名，如 alpha）
    Greek(String, Span),
    /// 常量
    Const(Const, Span),
    /// 素记号求导：f'(x)、y'' 中的撇
    Prime {
        base: Box<Expr>,
        order: u32,
        span: Span,
    },
    /// 函数调用 sin(x) / fn 声明后的 f(a,b) / mid(a,b)
    Call {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
    /// 一元运算
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    /// 二元运算（含隐式乘 Mul 无 * 字面）
    Binary {
        op: BinOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
        span: Span,
    },
    /// 上标
    Sup {
        base: Box<Expr>,
        exp: Box<Expr>,
        span: Span,
    },
    /// 下标
    Sub {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    /// |x| 绝对值
    Abs(Box<Expr>, Span),
    /// ||x|| 范数
    Norm(Box<Expr>, Span),
    /// 矩阵
    Matrix {
        kind: MatrixKind,
        rows: Vec<Vec<Expr>>,
        span: Span,
    },
    /// 分段函数 cases{ ... }
    Cases {
        branches: Vec<CaseBranch>,
        span: Span,
    },
    /// 对齐方程 align{ lhs &= rhs; ... }
    Align { eqs: Vec<AlignEq>, span: Span },
    /// 集合 {x mid x > 0}
    Set {
        elem: Box<Expr>,
        pred: Box<Expr>,
        span: Span,
    },
    /// "..." 直立文本
    Text(String, Span),
    /// 二项式 binom(n, k)
    Binom {
        n: Box<Expr>,
        k: Box<Expr>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub value: Expr,
    /// 条件表达式，或 Text("otherwise")
    pub cond: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlignEq {
    /// & 之前的左段（可为空表示续行）
    pub lhs: Option<Expr>,
    /// & = 之后的右段
    pub rhs: Expr,
}

/// fn 声明：登记 name 为函数，供 `a(b)` 调用/乘法消歧（规范 §5.1）。
#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: Option<Expr>,
    pub span: Span,
}

/// 一个公式的完整 AST 根。
#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    pub expr: Expr,
    pub fn_decls: Vec<FnDecl>,
}

impl fmt::Display for Const {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Const::Infinity => "oo",
            Const::Pi => "pi",
            Const::E => "e",
            Const::ImagI => "'i",
        })
    }
}
