#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Unit,
    Bool,
    U8, U16, U32, U64,
    I8, I16, I32, I64,
    F32, F64,
    Str,
    Ptr(Box<Ty>),
    Named(String),
}

#[derive(Debug, Clone)]
pub enum SemExpr {
    Unit,
    Bool(bool),
    Int(u64, Ty),
    Float(f64, Ty),
    Str(String),
    Var(String),
    Field(Box<SemExpr>, String),
    Call(String, Vec<SemExpr>),
    MethodCall(Box<SemExpr>, String, Vec<SemExpr>),
    Bin(BinOp, Box<SemExpr>, Box<SemExpr>, Ty),
    Un(UnOp, Box<SemExpr>, Ty),
    Cast(Box<SemExpr>, Ty),
    Deref(Box<SemExpr>),
    AddrOf(Box<SemExpr>),
    StructLit(String, Vec<(String, SemExpr)>),
    AsmInput(Box<SemExpr>),
}

use crate::ast::{BinOp, UnOp};

#[derive(Debug, Clone)]
pub enum SemAsm {
    Output(SemExpr, SemExpr),
    Input(SemExpr),
    Interrupt(SemExpr),
    Halt,
    NoOperation,
}

#[derive(Debug, Clone)]
pub enum SemStmt {
    Let(String, Ty, SemExpr),
    Assign(SemExpr, SemExpr),
    Return(Option<SemExpr>),
    If(SemExpr, Vec<SemStmt>, Vec<SemStmt>),
    While(SemExpr, Vec<SemStmt>),
    ForEach(String, SemExpr, Vec<SemStmt>),
    ExprStmt(SemExpr),
    Unsafe(Vec<SemStmt>),
    Assembly(Vec<SemAsm>),
    Panic(String),
    Emit(String),
    Show(SemExpr, String),
}

#[derive(Debug, Clone)]
pub struct SemFnSig {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub is_method: bool,
}

#[derive(Debug, Clone)]
pub struct SemFn {
    pub sig: SemFnSig,
    pub is_unsafe: bool,
    pub body: Vec<SemStmt>,
    pub self_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SemStruct {
    pub name: String,
    pub fields: Vec<(String, Ty, Option<SemExpr>)>,
}

#[derive(Debug, Clone)]
pub struct SemProtocol {
    pub name: String,
    pub methods: Vec<SemFnSig>,
}

#[derive(Debug, Clone)]
pub struct SemImpl {
    pub for_type: Ty,
    pub protocol: Option<String>,
    pub methods: Vec<SemFn>,
}

#[derive(Debug, Clone)]
pub struct SemRule {
    pub when_type: String,
    pub when_event: String,
    pub params: Vec<(String, Ty)>,
    pub body: Vec<SemStmt>,
}

#[derive(Debug, Clone)]
pub struct SemStatic {
    pub name: String,
    pub ty: Ty,
    pub mutable: bool,
    pub init: Option<SemExpr>,
}

#[derive(Debug, Clone)]
pub struct SemExtern {
    pub abi: String,
    pub sigs: Vec<SemFnSig>,
}

#[derive(Debug, Clone)]
pub struct SemModule {
    pub name: String,
    pub profile: crate::ast::Profile,
    pub no_standard: bool,
    pub entry: Option<String>,
    pub structs: Vec<SemStruct>,
    pub enums: Vec<(String, Vec<String>)>,
    pub protocols: Vec<SemProtocol>,
    pub impls: Vec<SemImpl>,
    pub fns: Vec<SemFn>,
    pub rules: Vec<SemRule>,
    pub statics: Vec<SemStatic>,
    pub externs: Vec<SemExtern>,
}

impl Ty {
    pub fn c_name(&self) -> String {
        match self {
            Ty::Unit => "void".into(),
            Ty::Bool => "bool".into(),
            Ty::U8  => "uint8_t".into(),
            Ty::U16 => "uint16_t".into(),
            Ty::U32 => "uint32_t".into(),
            Ty::U64 => "uint64_t".into(),
            Ty::I8  => "int8_t".into(),
            Ty::I16 => "int16_t".into(),
            Ty::I32 => "int32_t".into(),
            Ty::I64 => "int64_t".into(),
            Ty::F32 => "float".into(),
            Ty::F64 => "double".into(),
            Ty::Str => "uint8_t*".into(),
            Ty::Ptr(t) => format!("{}*", t.c_name()),
            Ty::Named(n) => n.clone(),
        }
    }

    pub fn js_zero(&self) -> &'static str {
        match self {
            Ty::Bool => "false",
            Ty::Str => "\"\"",
            _ => "0",
        }
    }
}