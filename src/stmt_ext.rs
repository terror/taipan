pub(crate) trait StmtExt {
  fn name(&self) -> &'static str;
}

impl StmtExt for ruff_python_ast::Stmt {
  fn name(&self) -> &'static str {
    match self {
      ruff_python_ast::Stmt::AnnAssign(_) => "AnnAssign",
      ruff_python_ast::Stmt::Assert(_) => "Assert",
      ruff_python_ast::Stmt::Assign(_) => "Assign",
      ruff_python_ast::Stmt::AugAssign(_) => "AugAssign",
      ruff_python_ast::Stmt::Break(_) => "Break",
      ruff_python_ast::Stmt::ClassDef(_) => "ClassDef",
      ruff_python_ast::Stmt::Continue(_) => "Continue",
      ruff_python_ast::Stmt::Delete(_) => "Delete",
      ruff_python_ast::Stmt::Expr(_) => "Expr",
      ruff_python_ast::Stmt::For(_) => "For",
      ruff_python_ast::Stmt::FunctionDef(_) => "FunctionDef",
      ruff_python_ast::Stmt::Global(_) => "Global",
      ruff_python_ast::Stmt::If(_) => "If",
      ruff_python_ast::Stmt::Import(_) => "Import",
      ruff_python_ast::Stmt::ImportFrom(_) => "ImportFrom",
      ruff_python_ast::Stmt::IpyEscapeCommand(_) => "IpyEscapeCommand",
      ruff_python_ast::Stmt::Match(_) => "Match",
      ruff_python_ast::Stmt::Nonlocal(_) => "Nonlocal",
      ruff_python_ast::Stmt::Pass(_) => "Pass",
      ruff_python_ast::Stmt::Raise(_) => "Raise",
      ruff_python_ast::Stmt::Return(_) => "Return",
      ruff_python_ast::Stmt::Try(_) => "Try",
      ruff_python_ast::Stmt::TypeAlias(_) => "TypeAlias",
      ruff_python_ast::Stmt::While(_) => "While",
      ruff_python_ast::Stmt::With(_) => "With",
    }
  }
}
