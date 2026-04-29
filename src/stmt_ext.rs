use super::*;

pub(crate) trait StmtExt {
  fn name(&self) -> &'static str;
}

impl StmtExt for Stmt {
  fn name(&self) -> &'static str {
    match self {
      Stmt::AnnAssign(_) => "AnnAssign",
      Stmt::Assert(_) => "Assert",
      Stmt::Assign(_) => "Assign",
      Stmt::AugAssign(_) => "AugAssign",
      Stmt::Break(_) => "Break",
      Stmt::ClassDef(_) => "ClassDef",
      Stmt::Continue(_) => "Continue",
      Stmt::Delete(_) => "Delete",
      Stmt::Expr(_) => "Expr",
      Stmt::For(_) => "For",
      Stmt::FunctionDef(_) => "FunctionDef",
      Stmt::Global(_) => "Global",
      Stmt::If(_) => "If",
      Stmt::Import(_) => "Import",
      Stmt::ImportFrom(_) => "ImportFrom",
      Stmt::IpyEscapeCommand(_) => "IpyEscapeCommand",
      Stmt::Match(_) => "Match",
      Stmt::Nonlocal(_) => "Nonlocal",
      Stmt::Pass(_) => "Pass",
      Stmt::Raise(_) => "Raise",
      Stmt::Return(_) => "Return",
      Stmt::Try(_) => "Try",
      Stmt::TypeAlias(_) => "TypeAlias",
      Stmt::While(_) => "While",
      Stmt::With(_) => "With",
    }
  }
}
