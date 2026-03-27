/// SpEL expression AST node.
///
/// The entry point is always `Expr`; evaluation produces a `Value`.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // ── literals ──────────────────────────────────────────────────────────
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),
    StringLit(String),
    Null,

    // ── property placeholder ──────────────────────────────────────────────
    /// `${key:default}` — resolved from the env map.
    PropertyPlaceholder {
        key: String,
        default: Option<String>,
    },

    // ── identifier / property reference ───────────────────────────────────
    /// Bare identifier: looks up by name in the env map.
    Identifier(String),

    // ── unary ─────────────────────────────────────────────────────────────
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    // ── binary ────────────────────────────────────────────────────────────
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    // ── ternary ───────────────────────────────────────────────────────────
    Ternary {
        cond: Box<Expr>,
        then_e: Box<Expr>,
        else_e: Box<Expr>,
    },

    // ── method call on an expression ──────────────────────────────────────
    /// `expr.method(args...)`
    MethodCall {
        target: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg, // -
    Not, // !
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}
