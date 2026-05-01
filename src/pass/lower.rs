use super::*;

pub(crate) struct Lower;

impl Lower {
  fn binary_operator(operator: Operator) -> Result<BinaryOperator> {
    match operator {
      Operator::Add => Ok(BinaryOperator::Add),
      Operator::Div => Ok(BinaryOperator::Div),
      Operator::FloorDiv => Ok(BinaryOperator::FloorDiv),
      Operator::Mod => Ok(BinaryOperator::Mod),
      Operator::Mult => Ok(BinaryOperator::Mul),
      Operator::Pow => Ok(BinaryOperator::Pow),
      Operator::Sub => Ok(BinaryOperator::Sub),
      _ => Err(Error::UnsupportedSyntax {
        message: format!("operator: {}", operator.name()),
      }),
    }
  }

  fn body(body: &[ruff_python_ast::Stmt]) -> Result<Vec<Stmt>> {
    body.iter().map(Self::stmt).collect()
  }

  fn call(node: &ruff_python_ast::ExprCall) -> Result<Expr> {
    if !node.arguments.keywords.is_empty() {
      return Err(Error::UnsupportedSyntax {
        message: "keyword arguments".into(),
      });
    }

    Ok(Expr::Call {
      arguments: node
        .arguments
        .args
        .iter()
        .map(Self::expr)
        .collect::<Result<Vec<_>>>()?,
      function: Box::new(Self::expr(&node.func)?),
    })
  }

  fn compare(node: &ruff_python_ast::ExprCompare) -> Result<Expr> {
    if node.ops.len() != 1 {
      return Err(Error::UnsupportedSyntax {
        message: "chained comparisons".into(),
      });
    }

    let rhs = node.comparators.first().ok_or_else(|| Error::Compile {
      message: "missing comparison operand".into(),
    })?;

    let operator = match node.ops[0] {
      CmpOp::Eq => CompareOperator::Eq,
      CmpOp::NotEq => CompareOperator::Ne,
      CmpOp::Lt => CompareOperator::Lt,
      CmpOp::LtE => CompareOperator::Le,
      CmpOp::Gt => CompareOperator::Gt,
      CmpOp::GtE => CompareOperator::Ge,
      CmpOp::Is | CmpOp::IsNot | CmpOp::In | CmpOp::NotIn => {
        return Err(Error::UnsupportedSyntax {
          message: format!("comparison operator: {}", node.ops[0].as_str()),
        });
      }
    };

    Ok(Expr::Compare {
      lhs: Box::new(Self::expr(&node.left)?),
      operator,
      rhs: Box::new(Self::expr(rhs)?),
    })
  }

  fn expr(expr: &ruff_python_ast::Expr) -> Result<Expr> {
    match expr {
      ruff_python_ast::Expr::BinOp(node) => Ok(Expr::Binary {
        lhs: Box::new(Self::expr(&node.left)?),
        operator: Self::binary_operator(node.op)?,
        rhs: Box::new(Self::expr(&node.right)?),
      }),
      ruff_python_ast::Expr::BoolOp(node) => Ok(Expr::BoolOp {
        operator: match node.op {
          BoolOp::And => BoolOperator::And,
          BoolOp::Or => BoolOperator::Or,
        },
        values: node.values.iter().map(Self::expr).collect::<Result<_>>()?,
      }),
      ruff_python_ast::Expr::BooleanLiteral(node) => Ok(Expr::Bool(node.value)),
      ruff_python_ast::Expr::Call(node) => Self::call(node),
      ruff_python_ast::Expr::Compare(node) => Self::compare(node),
      ruff_python_ast::Expr::If(node) => Ok(Expr::If {
        body: Box::new(Self::expr(&node.body)?),
        orelse: Box::new(Self::expr(&node.orelse)?),
        test: Box::new(Self::expr(&node.test)?),
      }),
      ruff_python_ast::Expr::Name(node) => Ok(Expr::Name(node.id.to_string())),
      ruff_python_ast::Expr::NoneLiteral(_) => Ok(Expr::None),
      ruff_python_ast::Expr::NumberLiteral(node) => Self::number(node),
      ruff_python_ast::Expr::StringLiteral(node) => {
        Ok(Expr::String(node.value.to_str().to_owned()))
      }
      ruff_python_ast::Expr::UnaryOp(node) => {
        let operator = match node.op {
          UnaryOp::USub => UnaryOperator::USub,
          UnaryOp::UAdd => UnaryOperator::UAdd,
          UnaryOp::Not => UnaryOperator::Not,
          UnaryOp::Invert => {
            return Err(Error::UnsupportedSyntax {
              message: "bitwise invert (~)".into(),
            });
          }
        };

        Ok(Expr::Unary {
          operand: Box::new(Self::expr(&node.operand)?),
          operator,
        })
      }
      _ => Err(Error::UnsupportedSyntax {
        message: format!("expression: {}", expr.name()),
      }),
    }
  }

  pub(crate) fn module(module: &ModModule) -> Result<Module> {
    Ok(Module {
      body: Self::body(&module.body)?,
    })
  }

  fn number(node: &ruff_python_ast::ExprNumberLiteral) -> Result<Expr> {
    match &node.value {
      Number::Int(int) => {
        Ok(Expr::Int(int.as_i64().ok_or_else(|| Error::Compile {
          message: "integer too large".into(),
        })?))
      }
      Number::Float(float) => Ok(Expr::Float(*float)),
      Number::Complex { .. } => Err(Error::UnsupportedSyntax {
        message: "complex numbers".into(),
      }),
    }
  }

  fn parameters(node: &ruff_python_ast::Parameters) -> Vec<String> {
    node
      .posonlyargs
      .iter()
      .chain(node.args.iter())
      .map(|argument| argument.parameter.name.id.to_string())
      .collect()
  }

  fn stmt(stmt: &ruff_python_ast::Stmt) -> Result<Stmt> {
    match stmt {
      ruff_python_ast::Stmt::AnnAssign(node) => Ok(Stmt::AnnAssign {
        target: Self::target(&node.target)?,
        value: node.value.as_deref().map(Self::expr).transpose()?,
      }),
      ruff_python_ast::Stmt::Assign(node) => Ok(Stmt::Assign {
        targets: node
          .targets
          .iter()
          .map(Self::target)
          .collect::<Result<Vec<_>>>()?,
        value: Self::expr(&node.value)?,
      }),
      ruff_python_ast::Stmt::AugAssign(node) => Ok(Stmt::AugAssign {
        operator: Self::binary_operator(node.op)?,
        target: Self::target(&node.target)?,
        value: Self::expr(&node.value)?,
      }),
      ruff_python_ast::Stmt::Break(_) => Ok(Stmt::Break),
      ruff_python_ast::Stmt::Continue(_) => Ok(Stmt::Continue),
      ruff_python_ast::Stmt::Expr(node) => {
        Ok(Stmt::Expr(Self::expr(&node.value)?))
      }
      ruff_python_ast::Stmt::FunctionDef(node) => {
        Ok(Stmt::FunctionDef(FunctionDef {
          body: Self::body(&node.body)?,
          name: node.name.id.to_string(),
          parameters: Self::parameters(&node.parameters),
        }))
      }
      ruff_python_ast::Stmt::Global(node) => Ok(Stmt::Global(
        node.names.iter().map(|name| name.id.to_string()).collect(),
      )),
      ruff_python_ast::Stmt::If(node) => Ok(Stmt::If {
        body: Self::body(&node.body)?,
        clauses: node
          .elif_else_clauses
          .iter()
          .map(|clause| {
            Ok((
              clause.test.as_ref().map(Self::expr).transpose()?,
              Self::body(&clause.body)?,
            ))
          })
          .collect::<Result<Vec<_>>>()?,
        test: Self::expr(&node.test)?,
      }),
      ruff_python_ast::Stmt::Nonlocal(node) => Ok(Stmt::Nonlocal(
        node.names.iter().map(|name| name.id.to_string()).collect(),
      )),
      ruff_python_ast::Stmt::Pass(_) => Ok(Stmt::Pass),
      ruff_python_ast::Stmt::Return(node) => Ok(Stmt::Return(
        node.value.as_deref().map(Self::expr).transpose()?,
      )),
      ruff_python_ast::Stmt::While(node) => Ok(Stmt::While {
        body: Self::body(&node.body)?,
        orelse: Self::body(&node.orelse)?,
        test: Self::expr(&node.test)?,
      }),
      _ => Err(Error::UnsupportedSyntax {
        message: format!("statement: {}", stmt.name()),
      }),
    }
  }

  fn target(target: &ruff_python_ast::Expr) -> Result<Expr> {
    match target {
      ruff_python_ast::Expr::Name(name) => Ok(Expr::Name(name.id.to_string())),
      _ => Err(Error::UnsupportedSyntax {
        message: "complex assignment target".into(),
      }),
    }
  }
}

impl Pass for Lower {
  fn run(&mut self, context: &mut Context<'_>) -> Result {
    context.set_ast(Self::module(context.module())?);
    Ok(())
  }
}
