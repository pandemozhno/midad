use crate::ast::Profile;
use crate::error::{Error, Result};
use crate::sem::*;

pub fn check(m: &SemModule) -> Result<()> {
    for f in &m.fns {
        check_fn(m.profile, f)?;
    }
    for imp in &m.impls {
        for f in &imp.methods {
            check_fn(m.profile, f)?;
        }
    }
    for r in &m.rules {
        if m.profile != Profile::Flow {
            return Err(Error::new(crate::error::ErrorKind::Checking,
                "rule разрешён только в @flow", 0, 0));
        }
        check_stmts(m.profile, &r.body)?;
    }
    Ok(())
}

fn check_fn(p: Profile, f: &SemFn) -> Result<()> {
    check_stmts(p, &f.body)
}

fn check_stmts(p: Profile, body: &[SemStmt]) -> Result<()> {
    for s in body { check_stmt(p, s)?; }
    Ok(())
}

fn check_stmt(p: Profile, s: &SemStmt) -> Result<()> {
    match s {
        SemStmt::Assembly(_) if p != Profile::Kernel => {
            Err(Error::new(crate::error::ErrorKind::Checking,
                "assembly разрешён только в @kernel", 0, 0))
        }
        SemStmt::Show(_, _) if p == Profile::Kernel => {
            Err(Error::new(crate::error::ErrorKind::Checking,
                "show не поддерживается в @kernel; используй внешний вывод", 0, 0))
        }
        SemStmt::Unsafe(_) if p != Profile::Kernel => {
            Err(Error::new(crate::error::ErrorKind::Checking,
                "unsafe разрешён только в @kernel", 0, 0))
        }
        SemStmt::If(_, t, e) => {
            check_stmts(p, t)?;
            check_stmts(p, e)
        }
        SemStmt::While(_, b) => check_stmts(p, b),
        SemStmt::ForEach(_, _, b) => check_stmts(p, b),
        SemStmt::Unsafe(b) => check_stmts(p, b),
        _ => Ok(()),
    }
}