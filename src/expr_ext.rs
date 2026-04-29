use super::*;

pub(crate) trait ExprExt {
  fn name(&self) -> &'static str;
}

impl ExprExt for Expr {
  fn name(&self) -> &'static str {
    match self {
      Expr::Attribute(_) => "Attribute",
      Expr::Await(_) => "Await",
      Expr::BinOp(_) => "BinOp",
      Expr::BoolOp(_) => "BoolOp",
      Expr::BooleanLiteral(_) => "BooleanLiteral",
      Expr::BytesLiteral(_) => "BytesLiteral",
      Expr::Call(_) => "Call",
      Expr::Compare(_) => "Compare",
      Expr::Dict(_) => "Dict",
      Expr::DictComp(_) => "DictComp",
      Expr::EllipsisLiteral(_) => "EllipsisLiteral",
      Expr::FString(_) => "FString",
      Expr::Generator(_) => "Generator",
      Expr::If(_) => "If",
      Expr::IpyEscapeCommand(_) => "IpyEscapeCommand",
      Expr::Lambda(_) => "Lambda",
      Expr::List(_) => "List",
      Expr::ListComp(_) => "ListComp",
      Expr::Name(_) => "Name",
      Expr::Named(_) => "Named",
      Expr::NoneLiteral(_) => "NoneLiteral",
      Expr::NumberLiteral(_) => "NumberLiteral",
      Expr::Set(_) => "Set",
      Expr::SetComp(_) => "SetComp",
      Expr::Slice(_) => "Slice",
      Expr::Starred(_) => "Starred",
      Expr::StringLiteral(_) => "StringLiteral",
      Expr::Subscript(_) => "Subscript",
      Expr::TString(_) => "TString",
      Expr::Tuple(_) => "Tuple",
      Expr::UnaryOp(_) => "UnaryOp",
      Expr::Yield(_) => "Yield",
      Expr::YieldFrom(_) => "YieldFrom",
    }
  }
}
