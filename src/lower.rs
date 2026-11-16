use crate::ast::{self, BinOp, Expr, Item, Stmt, TypeAst};
use crate::error::Result; 
use crate::sem::*;
use crate::token::TypeTok;

pub struct Lowerer {
    pub structs: Vec<(String, Vec<(String, Ty)>)>,
}

impl Lowerer {
    pub fn new() -> Self { Self { structs: Vec::new() } }

    pub fn lower(&mut self, m: ast::Module) -> Result<SemModule> {
        // первый проход — регистрируем структуры
        for it in &m.items {
            if let Item::Structure { name, fields } = it {
                let fields_c: Vec<(String, Ty)> = fields.iter()
                    .map(|(n, t, _)| (n.clone(), self.lower_type(t)))
                    .collect();
                self.structs.push((name.clone(), fields_c));
            }
        }

        let mut sm = SemModule {
            name: m.name,
            profile: m.profile,
            no_standard: m.no_standard,
            entry: m.entry,
            structs: Vec::new(),
            enums: Vec::new(),
            protocols: Vec::new(),
            impls: Vec::new(),
            fns: Vec::new(),
            rules: Vec::new(),
            statics: Vec::new(),
            externs: Vec::new(),
        };

        for it in m.items {
            match it {
                Item::Function { sig, is_unsafe, body } => {
                    let sfn = self.lower_fn(sig, is_unsafe, body, None)?;
                    sm.fns.push(sfn);
                }
                Item::Structure { name, fields } => {
                    let fields_c: Vec<(String, Ty, Option<SemExpr>)> = fields.iter()
                        .map(|(n, t, d)| {
                            let d_sem = match d {
                                Some(e) => Some(self.lower_expr(e.clone(), None)?),
                                None => None,
                            };
                            Ok((n.clone(), self.lower_type(t), d_sem))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    sm.structs.push(SemStruct { name, fields: fields_c });
                }
                Item::Enumeration { name, variants } => {
                    sm.enums.push((name, variants));
                }
                Item::Protocol { name, methods } => {
                    let sigs = methods.iter().map(|s| self.lower_sig(s.clone())).collect::<Result<Vec<_>>>()?;
                    sm.protocols.push(SemProtocol { name, methods: sigs });
                }
                Item::Implementation { for_type, protocol, methods } => {
                    let ft = self.lower_type(&for_type);
                    let mut ms = Vec::new();
                    for it in methods {
                        if let Item::Function { sig, is_unsafe, body } = it {
                            let self_name = if let Ty::Named(n) = &ft { Some(n.clone()) } else { None };
                            ms.push(self.lower_fn(sig, is_unsafe, body, self_name)?);
                        }
                    }
                    sm.impls.push(SemImpl { for_type: ft, protocol, methods: ms });
                }
                Item::Static { name, ty, mutable, init } => {
                    sm.statics.push(SemStatic {
                        name,
                        ty: self.lower_type(&ty),
                        mutable,
                        init: init.map(|e| self.lower_expr(e, None)).transpose()?,
                    });
                }
                Item::External { abi, sigs } => {
                    let ss = sigs.iter().map(|s| self.lower_sig(s.clone())).collect::<Result<Vec<_>>>()?;
                    sm.externs.push(SemExtern { abi, sigs: ss });
                }
                Item::Rule { when_type, when_event, params, body } => {
                    let ps: Vec<(String, Ty)> = params.iter()
                        .map(|(n, t)| (n.clone(), self.lower_type(t)))
                        .collect();
                    let bs: Vec<SemStmt> = body.iter()
                        .map(|s| self.lower_stmt(s.clone(), Some(&when_type)))
                        .collect::<Result<Vec<_>>>()?;
                    sm.rules.push(SemRule {
                        when_type, when_event, params: ps, body: bs,
                    });
                }
            }
        }
        Ok(sm)
    }

    fn lower_type(&self, t: &TypeAst) -> Ty {
        match t {
            TypeAst::Prim(TypeTok::Unsigned8)  => Ty::U8,
            TypeAst::Prim(TypeTok::Unsigned16) => Ty::U16,
            TypeAst::Prim(TypeTok::Unsigned32) => Ty::U32,
            TypeAst::Prim(TypeTok::Unsigned64) => Ty::U64,
            TypeAst::Prim(TypeTok::Integer8)   => Ty::I8,
            TypeAst::Prim(TypeTok::Integer16)  => Ty::I16,
            TypeAst::Prim(TypeTok::Integer32)  => Ty::I32,
            TypeAst::Prim(TypeTok::Integer64)  => Ty::I64,
            TypeAst::Prim(TypeTok::Float32)    => Ty::F32,
            TypeAst::Prim(TypeTok::Float64)    => Ty::F64,
            TypeAst::Prim(TypeTok::Boolean)    => Ty::Bool,
            TypeAst::Prim(TypeTok::Size)       => Ty::U64,
            TypeAst::Prim(TypeTok::String)     => Ty::Str,
            TypeAst::Pointer(inner)            => Ty::Ptr(Box::new(self.lower_type(inner))),
            TypeAst::Named(n)                  => Ty::Named(n.clone()),
        }
    }

    fn lower_sig(&self, s: ast::FnSig) -> Result<SemFnSig> {
        let params = s.params.iter()
            .map(|(n, t)| (n.clone(), self.lower_type(t)))
            .collect();
        Ok(SemFnSig {
            name: s.name,
            params,
            ret: s.ret.as_ref().map(|t| self.lower_type(t)).unwrap_or(Ty::Unit),
            is_method: s.is_method,
        })
    }

    fn lower_fn(&self, sig: ast::FnSig, is_unsafe: bool, body: Vec<Stmt>, self_type: Option<String>) -> Result<SemFn> {
        let sem_sig = self.lower_sig(sig)?;
        let sem_body = body.iter()
            .map(|s| self.lower_stmt(s.clone(), self_type.as_deref()))
            .collect::<Result<Vec<_>>>()?;
        Ok(SemFn { sig: sem_sig, is_unsafe, body: sem_body, self_type })
    }

    fn lower_stmt(&self, s: Stmt, self_type: Option<&str>) -> Result<SemStmt> {
        Ok(match s {
            Stmt::Let(n, t, e) => {
                let ty = if let Some(t) = t { self.lower_type(&t) } else { self.infer_type(&e) };
                SemStmt::Let(n, ty, self.lower_expr(e, self_type)?)
            }
            Stmt::Assign(t, v) => {
                let st = self.lower_expr(t, self_type)?;
                let sv = self.lower_expr(v, self_type)?;
                SemStmt::Assign(st, sv)
            }
            Stmt::Return(Some(e)) => SemStmt::Return(Some(self.lower_expr(e, self_type)?)),
            Stmt::Return(None) => SemStmt::Return(None),
            Stmt::If(c, t, e) => {
                let sc = self.lower_expr(c, self_type)?;
                let st = t.iter().map(|x| self.lower_stmt(x.clone(), self_type)).collect::<Result<Vec<_>>>()?;
                let se = e.iter().map(|x| self.lower_stmt(x.clone(), self_type)).collect::<Result<Vec<_>>>()?;
                SemStmt::If(sc, st, se)
            }
            Stmt::While(c, b) => {
                let sc = self.lower_expr(c, self_type)?;
                let sb = b.iter().map(|x| self.lower_stmt(x.clone(), self_type)).collect::<Result<Vec<_>>>()?;
                SemStmt::While(sc, sb)
            }
            Stmt::ForEach(v, i, b) => {
                let si = self.lower_expr(i, self_type)?;
                let sb = b.iter().map(|x| self.lower_stmt(x.clone(), self_type)).collect::<Result<Vec<_>>>()?;
                SemStmt::ForEach(v, si, sb)
            }
            Stmt::ExprStmt(e) => SemStmt::ExprStmt(self.lower_expr(e, self_type)?),
            Stmt::Unsafe(b) => {
                let sb = b.iter().map(|x| self.lower_stmt(x.clone(), self_type)).collect::<Result<Vec<_>>>()?;
                SemStmt::Unsafe(sb)
            }
            Stmt::Assembly(ops) => {
                let sops = ops.into_iter().map(|op| match op {
                    ast::AsmOp::Output(p, v) => Ok(SemAsm::Output(
                        self.lower_expr(p, self_type)?,
                        self.lower_expr(v, self_type)?)),
                    ast::AsmOp::Input(p) => Ok(SemAsm::Input(self.lower_expr(p, self_type)?)),
                    ast::AsmOp::Interrupt(n) => Ok(SemAsm::Interrupt(self.lower_expr(n, self_type)?)),
                    ast::AsmOp::Halt => Ok(SemAsm::Halt),
                    ast::AsmOp::NoOperation => Ok(SemAsm::NoOperation),
                }).collect::<Result<Vec<_>>>()?;
                SemStmt::Assembly(sops)
            }
            Stmt::Panic(m) => SemStmt::Panic(m),
            Stmt::Emit(e) => SemStmt::Emit(e),
            Stmt::Show(e, f) => SemStmt::Show(self.lower_expr(e, self_type)?, f),
        })
    }

    fn lower_expr(&self, e: Expr, self_type: Option<&str>) -> Result<SemExpr> {
        Ok(match e {
            Expr::Int(n) => {
                // если укладывается в i32 — по умолчанию I32
                if n <= i32::MAX as u64 { SemExpr::Int(n, Ty::I32) }
                else { SemExpr::Int(n, Ty::I64) }
            }
            Expr::Float(f) => SemExpr::Float(f, Ty::F64),
            Expr::Str(s) => SemExpr::Str(s),
            Expr::Bool(b) => SemExpr::Bool(b),
            Expr::Var(n) => SemExpr::Var(n),
            Expr::Field(b, f) => SemExpr::Field(Box::new(self.lower_expr(*b, self_type)?), f),
            Expr::Call(n, a) => {
                let sa = a.into_iter().map(|x| self.lower_expr(x, self_type)).collect::<Result<Vec<_>>>()?;
                SemExpr::Call(n, sa)
            }
            Expr::MethodCall(o, m, a) => {
                let so = self.lower_expr(*o, self_type)?;
                let sa = a.into_iter().map(|x| self.lower_expr(x, self_type)).collect::<Result<Vec<_>>>()?;
                SemExpr::MethodCall(Box::new(so), m, sa)
            }
            Expr::Bin(op, a, b) => {
                let sa = self.lower_expr(*a, self_type)?;
                let sb = self.lower_expr(*b, self_type)?;
                let ty = self.bin_type(op, &sa, &sb);
                SemExpr::Bin(op, Box::new(sa), Box::new(sb), ty)
            }
            Expr::Un(op, a) => {
                let sa = self.lower_expr(*a, self_type)?;
                SemExpr::Un(op, Box::new(sa), Ty::I32)
            }
            Expr::Cast(a, t) => {
                let sa = self.lower_expr(*a, self_type)?;
                SemExpr::Cast(Box::new(sa), self.lower_type(&t))
            }
            Expr::Deref(a) => SemExpr::Deref(Box::new(self.lower_expr(*a, self_type)?)),
            Expr::AddrOf(a) => SemExpr::AddrOf(Box::new(self.lower_expr(*a, self_type)?)),
            Expr::StructLit(n, f) => {
                let sf = f.into_iter().map(|(k, v)| Ok((k, self.lower_expr(v, self_type)?)))
                    .collect::<Result<Vec<_>>>()?;
                SemExpr::StructLit(n, sf)
            }
            Expr::AsmInput(p) => SemExpr::AsmInput(Box::new(self.lower_expr(*p, self_type)?)),
        })
    }

    fn infer_type(&self, e: &Expr) -> Ty {
        match e {
            Expr::Int(n) => if *n <= i32::MAX as u64 { Ty::I32 } else { Ty::I64 },
            Expr::Float(_) => Ty::F64,
            Expr::Str(_) => Ty::Str,
            Expr::Bool(_) => Ty::Bool,
            Expr::StructLit(n, _) => Ty::Named(n.clone()),
            _ => Ty::I32,
        }
    }

    fn bin_type(&self, op: BinOp, a: &SemExpr, b: &SemExpr) -> Ty {
        match op {
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Gt
            | BinOp::Le | BinOp::Ge | BinOp::And | BinOp::Or => Ty::Bool,
            _ => {
                let ta = expr_ty(a);
                let tb = expr_ty(b);
                // простая унификация
                if ta == tb { ta } else if matches!(ta, Ty::Ptr(_)) { ta }
                else if matches!(tb, Ty::Ptr(_)) { tb }
                else { Ty::I32 }
            }
        }
    }
}

fn expr_ty(e: &SemExpr) -> Ty {
    match e {
        SemExpr::Int(_, t) => t.clone(),
        SemExpr::Float(_, t) => t.clone(),
        SemExpr::Str(_) => Ty::Str,
        SemExpr::Bool(_) => Ty::Bool,
        SemExpr::Bin(_, _, _, t) => t.clone(),
        SemExpr::Un(_, _, t) => t.clone(),
        SemExpr::Cast(_, t) => t.clone(),
        SemExpr::AsmInput(_) => Ty::U8,
        _ => Ty::I32,
    }
}