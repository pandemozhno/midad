#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Profile {
    Kernel, Flow, View, Light,
}

#[derive(Debug, Clone)]
pub enum TypeAst {
    Prim(TypeTok),
    Pointer(Box<TypeAst>),
    Named(String),
}

use crate::token::TypeTok;

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Gt, Le, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
}

#[derive(Debug, Clone, Copy)]
pub enum UnOp {
    Not, BitNot, Neg,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(u64),
    Float(f64),
    Str(String),
    Bool(bool),
    Var(String),
    Field(Box<Expr>, String),
    Call(String, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Un(UnOp, Box<Expr>),
    Cast(Box<Expr>, TypeAst),
    Deref(Box<Expr>),
    AddrOf(Box<Expr>),
    StructLit(String, Vec<(String, Expr)>),
    AsmInput(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum AsmOp {
    Output(Expr, Expr),
    Input(Expr),
    Interrupt(Expr),
    Halt,
    NoOperation,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let(String, Option<TypeAst>, Expr),
    Assign(Expr, Expr),
    Return(Option<Expr>),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    ForEach(String, Expr, Vec<Stmt>),
    ExprStmt(Expr),
    Unsafe(Vec<Stmt>),
    Assembly(Vec<AsmOp>),
    Panic(String),
    Emit(String),
    Show(Expr, String),
}

#[derive(Debug, Clone)]
pub struct FnSig {
    pub name: String,
    pub params: Vec<(String, TypeAst)>,
    pub ret: Option<TypeAst>,
    pub is_method: bool,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function {
        sig: FnSig,
        is_unsafe: bool,
        body: Vec<Stmt>,
    },
    Structure {
        name: String,
        fields: Vec<(String, TypeAst, Option<Expr>)>,
    },
    Enumeration {
        name: String,
        variants: Vec<String>,
    },
    Protocol {
        name: String,
        methods: Vec<FnSig>,
    },
    Implementation {
        for_type: TypeAst,
        protocol: Option<String>,
        methods: Vec<Item>,
    },
    Static {
        name: String,
        ty: TypeAst,
        mutable: bool,
        init: Option<Expr>,
    },
    External {
        abi: String,
        sigs: Vec<FnSig>,
    },
    Rule {
        when_type: String,
        when_event: String,
        params: Vec<(String, TypeAst)>,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub profile: Profile,
    pub no_standard: bool,
    pub entry: Option<String>,
    pub items: Vec<Item>,
}