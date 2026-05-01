use super::*;

pub(crate) struct ValidateSyntax;

impl ValidateSyntax {
  fn body(body: &[Stmt]) -> Result {
    for stmt in body {
      Self::stmt(stmt)?;
    }

    Ok(())
  }

  fn call(node: &ExprCall) -> Result {
    if !node.arguments.keywords.is_empty() {
      return Err(Error::UnsupportedSyntax {
        message: "keyword arguments".into(),
      });
    }

    Self::expr(&node.func)?;

    for argument in &*node.arguments.args {
      Self::expr(argument)?;
    }

    Ok(())
  }

  fn compare(node: &ExprCompare) -> Result {
    if node.ops.len() != 1 {
      return Err(Error::UnsupportedSyntax {
        message: "chained comparisons".into(),
      });
    }

    Self::expr(&node.left)?;
    Self::expr(&node.comparators[0])?;

    match node.ops[0] {
      CmpOp::Eq
      | CmpOp::NotEq
      | CmpOp::Lt
      | CmpOp::LtE
      | CmpOp::Gt
      | CmpOp::GtE => Ok(()),
      CmpOp::Is | CmpOp::IsNot | CmpOp::In | CmpOp::NotIn => {
        Err(Error::UnsupportedSyntax {
          message: format!("comparison operator: {}", node.ops[0].as_str()),
        })
      }
    }
  }

  fn expr(expr: &Expr) -> Result {
    match expr {
      Expr::BinOp(node) => {
        node.op.instruction()?;
        Self::expr(&node.left)?;
        Self::expr(&node.right)
      }
      Expr::BoolOp(node) => {
        for value in &node.values {
          Self::expr(value)?;
        }

        Ok(())
      }
      Expr::BooleanLiteral(_)
      | Expr::Name(_)
      | Expr::NoneLiteral(_)
      | Expr::StringLiteral(_) => Ok(()),
      Expr::Call(node) => Self::call(node),
      Expr::Compare(node) => Self::compare(node),
      Expr::If(node) => {
        Self::expr(&node.test)?;
        Self::expr(&node.body)?;
        Self::expr(&node.orelse)
      }
      Expr::NumberLiteral(node) => Self::number(node),
      Expr::UnaryOp(node) => {
        Self::expr(&node.operand)?;

        match node.op {
          UnaryOp::USub | UnaryOp::UAdd | UnaryOp::Not => Ok(()),
          UnaryOp::Invert => Err(Error::UnsupportedSyntax {
            message: "bitwise invert (~)".into(),
          }),
        }
      }
      _ => Err(Error::UnsupportedSyntax {
        message: format!("expression: {}", expr.name()),
      }),
    }
  }

  fn number(node: &ExprNumberLiteral) -> Result {
    match node.value {
      Number::Int(_) | Number::Float(_) => Ok(()),
      Number::Complex { .. } => Err(Error::UnsupportedSyntax {
        message: "complex numbers".into(),
      }),
    }
  }

  fn stmt(stmt: &Stmt) -> Result {
    match stmt {
      Stmt::AnnAssign(node) => {
        if let Some(value) = &node.value {
          Self::expr(value)?;
          Self::target(&node.target)?;
        }

        Ok(())
      }
      Stmt::Assign(node) => {
        Self::expr(&node.value)?;

        for target in &node.targets {
          Self::target(target)?;
        }

        Ok(())
      }
      Stmt::AugAssign(node) => {
        Self::target(&node.target)?;
        Self::expr(&node.value)
      }
      Stmt::Break(_)
      | Stmt::Continue(_)
      | Stmt::Global(_)
      | Stmt::Nonlocal(_)
      | Stmt::Pass(_) => Ok(()),
      Stmt::Expr(node) => Self::expr(&node.value),
      Stmt::FunctionDef(node) => Self::body(&node.body),
      Stmt::If(node) => {
        Self::expr(&node.test)?;
        Self::body(&node.body)?;

        for clause in &node.elif_else_clauses {
          if let Some(test) = &clause.test {
            Self::expr(test)?;
          }

          Self::body(&clause.body)?;
        }

        Ok(())
      }
      Stmt::Return(node) => {
        if let Some(value) = &node.value {
          Self::expr(value)?;
        }

        Ok(())
      }
      Stmt::While(node) => {
        Self::expr(&node.test)?;
        Self::body(&node.body)?;
        Self::body(&node.orelse)
      }
      _ => Err(Error::UnsupportedSyntax {
        message: format!("statement: {}", stmt.name()),
      }),
    }
  }

  fn target(target: &Expr) -> Result {
    match target {
      Expr::Name(_) => Ok(()),
      _ => Err(Error::UnsupportedSyntax {
        message: "complex assignment target".into(),
      }),
    }
  }
}

impl Pass for ValidateSyntax {
  fn run(&mut self, context: &mut Context<'_>) -> Result {
    Self::body(context.body())
  }
}
