use super::*;

pub(crate) struct LowerAst<'a> {
  source: &'a str,
}

impl<'a> LowerAst<'a> {
  fn binary_operator(operator: Operator) -> Result<BinaryOperator> {
    match operator {
      Operator::Add => Ok(BinaryOperator::Add),
      Operator::BitAnd => Ok(BinaryOperator::BitAnd),
      Operator::BitOr => Ok(BinaryOperator::BitOr),
      Operator::BitXor => Ok(BinaryOperator::BitXor),
      Operator::Div => Ok(BinaryOperator::Div),
      Operator::FloorDiv => Ok(BinaryOperator::FloorDiv),
      Operator::LShift => Ok(BinaryOperator::LShift),
      Operator::Mod => Ok(BinaryOperator::Mod),
      Operator::Mult => Ok(BinaryOperator::Mul),
      Operator::Pow => Ok(BinaryOperator::Pow),
      Operator::RShift => Ok(BinaryOperator::RShift),
      Operator::Sub => Ok(BinaryOperator::Sub),
      Operator::MatMult => Err(Error::UnsupportedSyntax {
        message: format!("operator: {}", operator.name()),
      }),
    }
  }

  fn body(&self, body: &[ruff_python_ast::Stmt]) -> Result<Vec<Stmt>> {
    body.iter().map(|stmt| self.stmt(stmt)).collect()
  }

  fn call(&self, node: &ruff_python_ast::ExprCall) -> Result<Expr> {
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
        .map(|expr| self.expr(expr))
        .collect::<Result<Vec<_>>>()?,
      function: Box::new(self.expr(&node.func)?),
    })
  }

  fn compare(&self, node: &ruff_python_ast::ExprCompare) -> Result<Expr> {
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
      CmpOp::In => CompareOperator::In,
      CmpOp::NotIn => CompareOperator::NotIn,
      CmpOp::Is | CmpOp::IsNot => {
        return Err(Error::UnsupportedSyntax {
          message: format!("comparison operator: {}", node.ops[0].as_str()),
        });
      }
    };

    Ok(Expr::Compare {
      lhs: Box::new(self.expr(&node.left)?),
      operator,
      rhs: Box::new(self.expr(rhs)?),
    })
  }

  fn debug_text(
    &self,
    interpolation: &ruff_python_ast::InterpolatedElement,
  ) -> Result<String> {
    let debug_text =
      interpolation
        .debug_text
        .as_ref()
        .ok_or_else(|| Error::Internal {
          message: "missing f-string debug text".into(),
        })?;

    let expression = self
      .source
      .get(interpolation.expression.range().to_std_range())
      .ok_or_else(|| Error::Internal {
        message: "invalid f-string debug expression range".into(),
      })?;

    Ok(format!(
      "{}{}{}",
      debug_text.leading, expression, debug_text.trailing
    ))
  }

  fn expr(&self, expr: &ruff_python_ast::Expr) -> Result<Expr> {
    match expr {
      ruff_python_ast::Expr::BinOp(node) => Ok(Expr::Binary {
        lhs: Box::new(self.expr(&node.left)?),
        operator: Self::binary_operator(node.op)?,
        rhs: Box::new(self.expr(&node.right)?),
      }),
      ruff_python_ast::Expr::BoolOp(node) => Ok(Expr::BoolOp {
        operator: match node.op {
          BoolOp::And => BoolOperator::And,
          BoolOp::Or => BoolOperator::Or,
        },
        values: node
          .values
          .iter()
          .map(|expr| self.expr(expr))
          .collect::<Result<_>>()?,
      }),
      ruff_python_ast::Expr::BooleanLiteral(node) => Ok(Expr::Bool(node.value)),
      ruff_python_ast::Expr::Call(node) => self.call(node),
      ruff_python_ast::Expr::Compare(node) => self.compare(node),
      ruff_python_ast::Expr::FString(node) => {
        self.formatted_string(node.value.iter())
      }
      ruff_python_ast::Expr::If(node) => Ok(Expr::If {
        body: Box::new(self.expr(&node.body)?),
        orelse: Box::new(self.expr(&node.orelse)?),
        test: Box::new(self.expr(&node.test)?),
      }),
      ruff_python_ast::Expr::List(node) => Ok(Expr::List(
        node
          .elts
          .iter()
          .map(|expr| self.expr(expr))
          .collect::<Result<_>>()?,
      )),
      ruff_python_ast::Expr::Name(node) => Ok(Expr::Name(node.id.to_string())),
      ruff_python_ast::Expr::NoneLiteral(_) => Ok(Expr::None),
      ruff_python_ast::Expr::NumberLiteral(node) => Self::number(node),
      ruff_python_ast::Expr::StringLiteral(node) => {
        Ok(Expr::String(node.value.to_str().to_owned()))
      }
      ruff_python_ast::Expr::Subscript(node) => Ok(Expr::Subscript {
        slice: Box::new(self.expr(&node.slice)?),
        value: Box::new(self.expr(&node.value)?),
      }),
      ruff_python_ast::Expr::Tuple(node) => Ok(Expr::Tuple(
        node
          .elts
          .iter()
          .map(|expr| self.expr(expr))
          .collect::<Result<_>>()?,
      )),
      ruff_python_ast::Expr::UnaryOp(node) => {
        let operator = match node.op {
          UnaryOp::USub => UnaryOperator::USub,
          UnaryOp::UAdd => UnaryOperator::UAdd,
          UnaryOp::Not => UnaryOperator::Not,
          UnaryOp::Invert => UnaryOperator::Invert,
        };

        Ok(Expr::Unary {
          operand: Box::new(self.expr(&node.operand)?),
          operator,
        })
      }
      _ => Err(Error::UnsupportedSyntax {
        message: format!("expression: {}", expr.name()),
      }),
    }
  }

  fn formatted_string<'b>(
    &self,
    parts: impl IntoIterator<Item = &'b FStringPart>,
  ) -> Result<Expr> {
    let mut expressions = Vec::new();

    for part in parts {
      match part {
        FStringPart::Literal(literal) => {
          expressions.push(Expr::String(literal.value.to_string()));
        }
        FStringPart::FString(fstring) => {
          for element in &fstring.elements {
            match element {
              InterpolatedStringElement::Literal(literal) => {
                expressions.push(Expr::String(literal.value.to_string()));
              }
              InterpolatedStringElement::Interpolation(interpolation) => {
                if interpolation.format_spec.is_some() {
                  return Err(Error::UnsupportedSyntax {
                    message: "f-string format spec".into(),
                  });
                }

                match interpolation.conversion {
                  ConversionFlag::None
                  | ConversionFlag::Str
                  | ConversionFlag::Repr => {
                    let expr = self.expr(&interpolation.expression)?;

                    if interpolation.debug_text.is_some() {
                      expressions
                        .push(Expr::String(self.debug_text(interpolation)?));
                    }

                    expressions.push(expr);
                  }
                  ConversionFlag::Ascii => {
                    return Err(Error::UnsupportedSyntax {
                      message: "f-string ascii conversion".into(),
                    });
                  }
                }
              }
            }
          }
        }
      }
    }

    Ok(Expr::FormattedString(expressions))
  }

  pub(crate) fn module(&self, module: &ModModule) -> Result<Module> {
    Ok(Module {
      body: self.body(&module.body)?,
    })
  }

  pub(crate) fn new(source: &'a str) -> Self {
    Self { source }
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

  fn parameters(
    &self,
    node: &ruff_python_ast::Parameters,
  ) -> Result<Vec<FunctionParameter>> {
    if node.vararg.is_some() {
      return Err(Error::UnsupportedSyntax {
        message: "variadic positional parameters".into(),
      });
    }

    if !node.kwonlyargs.is_empty() {
      return Err(Error::UnsupportedSyntax {
        message: "keyword-only parameters".into(),
      });
    }

    if node.kwarg.is_some() {
      return Err(Error::UnsupportedSyntax {
        message: "variadic keyword parameters".into(),
      });
    }

    node
      .posonlyargs
      .iter()
      .chain(node.args.iter())
      .map(|argument| {
        Ok(FunctionParameter {
          default: argument
            .default
            .as_deref()
            .map(|expr| self.expr(expr))
            .transpose()?,
          name: argument.parameter.name.id.to_string(),
        })
      })
      .collect()
  }

  fn stmt(&self, stmt: &ruff_python_ast::Stmt) -> Result<Stmt> {
    match stmt {
      ruff_python_ast::Stmt::AnnAssign(node) => Ok(Stmt::AnnAssign {
        target: self.target(&node.target)?,
        value: node
          .value
          .as_deref()
          .map(|expr| self.expr(expr))
          .transpose()?,
      }),
      ruff_python_ast::Stmt::Assign(node) => Ok(Stmt::Assign {
        targets: node
          .targets
          .iter()
          .map(|target| self.target(target))
          .collect::<Result<Vec<_>>>()?,
        value: self.expr(&node.value)?,
      }),
      ruff_python_ast::Stmt::AugAssign(node) => Ok(Stmt::AugAssign {
        operator: Self::binary_operator(node.op)?,
        target: self.target(&node.target)?,
        value: self.expr(&node.value)?,
      }),
      ruff_python_ast::Stmt::Break(_) => Ok(Stmt::Break),
      ruff_python_ast::Stmt::Continue(_) => Ok(Stmt::Continue),
      ruff_python_ast::Stmt::Expr(node) => {
        Ok(Stmt::Expr(self.expr(&node.value)?))
      }
      ruff_python_ast::Stmt::For(node) => {
        if node.is_async {
          return Err(Error::UnsupportedSyntax {
            message: "async for".into(),
          });
        }

        Ok(Stmt::For {
          body: self.body(&node.body)?,
          iter: self.expr(&node.iter)?,
          orelse: self.body(&node.orelse)?,
          target: self.target(&node.target)?,
        })
      }
      ruff_python_ast::Stmt::FunctionDef(node) => {
        Ok(Stmt::FunctionDef(FunctionDef {
          body: self.body(&node.body)?,
          name: node.name.id.to_string(),
          parameters: self.parameters(&node.parameters)?,
        }))
      }
      ruff_python_ast::Stmt::Global(node) => Ok(Stmt::Global(
        node.names.iter().map(|name| name.id.to_string()).collect(),
      )),
      ruff_python_ast::Stmt::If(node) => Ok(Stmt::If {
        body: self.body(&node.body)?,
        clauses: node
          .elif_else_clauses
          .iter()
          .map(|clause| {
            Ok((
              clause
                .test
                .as_ref()
                .map(|expr| self.expr(expr))
                .transpose()?,
              self.body(&clause.body)?,
            ))
          })
          .collect::<Result<Vec<_>>>()?,
        test: self.expr(&node.test)?,
      }),
      ruff_python_ast::Stmt::Nonlocal(node) => Ok(Stmt::Nonlocal(
        node.names.iter().map(|name| name.id.to_string()).collect(),
      )),
      ruff_python_ast::Stmt::Pass(_) => Ok(Stmt::Pass),
      ruff_python_ast::Stmt::Return(node) => Ok(Stmt::Return(
        node
          .value
          .as_deref()
          .map(|expr| self.expr(expr))
          .transpose()?,
      )),
      ruff_python_ast::Stmt::While(node) => Ok(Stmt::While {
        body: self.body(&node.body)?,
        orelse: self.body(&node.orelse)?,
        test: self.expr(&node.test)?,
      }),
      _ => Err(Error::UnsupportedSyntax {
        message: format!("statement: {}", stmt.name()),
      }),
    }
  }

  fn target(&self, target: &ruff_python_ast::Expr) -> Result<Expr> {
    match target {
      ruff_python_ast::Expr::List(node) => Ok(Expr::List(
        node
          .elts
          .iter()
          .map(|expr| self.target(expr))
          .collect::<Result<_>>()?,
      )),
      ruff_python_ast::Expr::Name(name) => Ok(Expr::Name(name.id.to_string())),
      ruff_python_ast::Expr::Subscript(node) => Ok(Expr::Subscript {
        slice: Box::new(self.expr(&node.slice)?),
        value: Box::new(self.expr(&node.value)?),
      }),
      ruff_python_ast::Expr::Tuple(node) => Ok(Expr::Tuple(
        node
          .elts
          .iter()
          .map(|expr| self.target(expr))
          .collect::<Result<_>>()?,
      )),
      _ => Err(Error::UnsupportedSyntax {
        message: "complex assignment target".into(),
      }),
    }
  }
}

impl Pass for LowerAst<'_> {
  fn run(&mut self, context: &mut Context<'_>) -> Result {
    context.set_ast(self.module(context.syntax())?);
    Ok(())
  }
}
