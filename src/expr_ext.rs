pub(crate) trait ExprExt {
  fn name(&self) -> &'static str;
}

impl ExprExt for ruff_python_ast::Expr {
  fn name(&self) -> &'static str {
    match self {
      ruff_python_ast::Expr::Attribute(_) => "Attribute",
      ruff_python_ast::Expr::Await(_) => "Await",
      ruff_python_ast::Expr::BinOp(_) => "BinOp",
      ruff_python_ast::Expr::BoolOp(_) => "BoolOp",
      ruff_python_ast::Expr::BooleanLiteral(_) => "BooleanLiteral",
      ruff_python_ast::Expr::BytesLiteral(_) => "BytesLiteral",
      ruff_python_ast::Expr::Call(_) => "Call",
      ruff_python_ast::Expr::Compare(_) => "Compare",
      ruff_python_ast::Expr::Dict(_) => "Dict",
      ruff_python_ast::Expr::DictComp(_) => "DictComp",
      ruff_python_ast::Expr::EllipsisLiteral(_) => "EllipsisLiteral",
      ruff_python_ast::Expr::FString(_) => "FString",
      ruff_python_ast::Expr::Generator(_) => "Generator",
      ruff_python_ast::Expr::If(_) => "If",
      ruff_python_ast::Expr::IpyEscapeCommand(_) => "IpyEscapeCommand",
      ruff_python_ast::Expr::Lambda(_) => "Lambda",
      ruff_python_ast::Expr::List(_) => "List",
      ruff_python_ast::Expr::ListComp(_) => "ListComp",
      ruff_python_ast::Expr::Name(_) => "Name",
      ruff_python_ast::Expr::Named(_) => "Named",
      ruff_python_ast::Expr::NoneLiteral(_) => "NoneLiteral",
      ruff_python_ast::Expr::NumberLiteral(_) => "NumberLiteral",
      ruff_python_ast::Expr::Set(_) => "Set",
      ruff_python_ast::Expr::SetComp(_) => "SetComp",
      ruff_python_ast::Expr::Slice(_) => "Slice",
      ruff_python_ast::Expr::Starred(_) => "Starred",
      ruff_python_ast::Expr::StringLiteral(_) => "StringLiteral",
      ruff_python_ast::Expr::Subscript(_) => "Subscript",
      ruff_python_ast::Expr::TString(_) => "TString",
      ruff_python_ast::Expr::Tuple(_) => "Tuple",
      ruff_python_ast::Expr::UnaryOp(_) => "UnaryOp",
      ruff_python_ast::Expr::Yield(_) => "Yield",
      ruff_python_ast::Expr::YieldFrom(_) => "YieldFrom",
    }
  }
}
