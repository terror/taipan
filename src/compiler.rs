use super::*;

pub struct Compiler {
  scopes: Vec<Scope>,
}

impl Compiler {
  fn add_const(&mut self, obj: Object) -> u16 {
    let code = &mut self.scope_mut().code;

    let idx = code.constants.len();

    code.constants.push(obj);

    u16::try_from(idx).expect("constant pool overflow")
  }

  fn add_local(&mut self, name: &str) -> u16 {
    let code = &mut self.scope_mut().code;

    if let Some(idx) = code.locals.iter().position(|n| n == name) {
      return u16::try_from(idx).expect("local table overflow");
    }

    let idx = code.locals.len();

    code.locals.push(name.to_owned());

    u16::try_from(idx).expect("local table overflow")
  }

  fn add_name(&mut self, name: &str) -> u16 {
    let code = &mut self.scope_mut().code;

    if let Some(idx) = code.names.iter().position(|n| n == name) {
      return u16::try_from(idx).expect("name table overflow");
    }

    let idx = code.names.len();

    code.names.push(name.to_owned());

    u16::try_from(idx).expect("name table overflow")
  }

  pub fn compile(module: &ModModule) -> Result<Code> {
    let mut compiler = Self {
      scopes: vec![Scope {
        code: Code::default(),
        in_function: false,
      }],
    };

    compiler.compile_body(&module.body)?;

    Ok(compiler.scopes.pop().unwrap().code)
  }

  fn compile_assign(&mut self, node: &StmtAssign) -> Result<()> {
    self.compile_expr(&node.value)?;

    for (i, target) in node.targets.iter().enumerate() {
      if i < node.targets.len() - 1 {
        self.emit(Op::Dup);
      }

      self.compile_store(target)?;
    }

    Ok(())
  }

  fn compile_aug_assign(&mut self, node: &StmtAugAssign) -> Result<()> {
    self.compile_load_target(&node.target)?;
    self.compile_expr(&node.value)?;
    self.emit(operator_to_binary_op(node.op)?);
    self.compile_store(&node.target)?;
    Ok(())
  }

  fn compile_body(&mut self, body: &[Stmt]) -> Result<()> {
    for stmt in body {
      self.compile_stmt(stmt)?;
    }

    Ok(())
  }

  fn compile_bool_op(&mut self, node: &ExprBoolOp) -> Result<()> {
    let is_and = matches!(node.op, BoolOp::And);

    let mut jumps = vec![];

    for (i, value) in node.values.iter().enumerate() {
      self.compile_expr(value)?;

      if i < node.values.len() - 1 {
        self.emit(Op::Dup);

        let jump = if is_and {
          self.emit_jump(Op::PopJumpIfFalse(0))
        } else {
          self.emit_jump(Op::PopJumpIfTrue(0))
        };

        self.emit(Op::Pop);

        jumps.push(jump);
      }
    }

    for jump in jumps {
      self.patch_jump(jump);
    }

    Ok(())
  }

  fn compile_call(&mut self, node: &ExprCall) -> Result<()> {
    if !node.arguments.keywords.is_empty() {
      return Err(Error::UnsupportedSyntax {
        message: "keyword arguments".into(),
      });
    }

    self.compile_expr(&node.func)?;

    let argc = node.arguments.args.len();

    for arg in &*node.arguments.args {
      self.compile_expr(arg)?;
    }

    let argc = u8::try_from(argc).map_err(|_| Error::Compile {
      message: "too many arguments".into(),
    })?;

    self.emit(Op::CallFunction(argc));

    Ok(())
  }

  fn compile_compare(&mut self, node: &ExprCompare) -> Result<()> {
    if node.ops.len() != 1 {
      return Err(Error::UnsupportedSyntax {
        message: "chained comparisons".into(),
      });
    }

    self.compile_expr(&node.left)?;

    self.compile_expr(&node.comparators[0])?;

    let op = match node.ops[0] {
      CmpOp::Eq => Op::CompareEq,
      CmpOp::NotEq => Op::CompareNe,
      CmpOp::Lt => Op::CompareLt,
      CmpOp::LtE => Op::CompareLe,
      CmpOp::Gt => Op::CompareGt,
      CmpOp::GtE => Op::CompareGe,
      CmpOp::Is | CmpOp::IsNot | CmpOp::In | CmpOp::NotIn => {
        return Err(Error::UnsupportedSyntax {
          message: format!("comparison operator: {}", node.ops[0].as_str()),
        });
      }
    };

    self.emit(op);

    Ok(())
  }

  fn compile_expr(&mut self, expr: &Expr) -> Result<()> {
    match expr {
      Expr::BinOp(node) => {
        self.compile_expr(&node.left)?;
        self.compile_expr(&node.right)?;
        self.emit(operator_to_binary_op(node.op)?);
        Ok(())
      }
      Expr::BoolOp(node) => self.compile_bool_op(node),
      Expr::BooleanLiteral(node) => {
        let idx = self.add_const(Object::Bool(node.value));
        self.emit(Op::LoadConst(idx));
        Ok(())
      }
      Expr::Call(node) => self.compile_call(node),
      Expr::Compare(node) => self.compile_compare(node),
      Expr::If(node) => {
        self.compile_expr(&node.test)?;
        let false_jump = self.emit_jump(Op::PopJumpIfFalse(0));
        self.compile_expr(&node.body)?;
        let end_jump = self.emit_jump(Op::Jump(0));
        self.patch_jump(false_jump);
        self.compile_expr(&node.orelse)?;
        self.patch_jump(end_jump);
        Ok(())
      }
      Expr::Name(node) => {
        let op = self.resolve_load(&node.id);
        self.emit(op);
        Ok(())
      }
      Expr::NoneLiteral(_) => {
        let idx = self.add_const(Object::None);
        self.emit(Op::LoadConst(idx));
        Ok(())
      }
      Expr::NumberLiteral(node) => self.compile_number(node),
      Expr::StringLiteral(node) => {
        let s = node.value.to_str().to_owned();
        let idx = self.add_const(Object::Str(s));
        self.emit(Op::LoadConst(idx));
        Ok(())
      }
      Expr::UnaryOp(node) => {
        self.compile_expr(&node.operand)?;
        match node.op {
          UnaryOp::USub => self.emit(Op::UnaryNeg),
          UnaryOp::UAdd => self.emit(Op::UnaryPos),
          UnaryOp::Not => self.emit(Op::UnaryNot),
          UnaryOp::Invert => {
            return Err(Error::UnsupportedSyntax {
              message: "bitwise invert (~)".into(),
            });
          }
        }
        Ok(())
      }
      _ => Err(Error::UnsupportedSyntax {
        message: format!("expression: {}", expr_name(expr)),
      }),
    }
  }

  fn compile_function_def(&mut self, node: &StmtFunctionDef) -> Result<()> {
    let params = node
      .parameters
      .args
      .iter()
      .chain(node.parameters.posonlyargs.iter())
      .map(|p| p.parameter.name.id.to_string())
      .collect::<Vec<_>>();

    self.scopes.push(Scope {
      code: Code::default(),
      in_function: true,
    });

    {
      let scope = self.scopes.last_mut().unwrap();

      for param in &params {
        scope.code.locals.push(param.clone());
      }
    }

    self.compile_body(&node.body)?;

    let last_is_return = self
      .scope()
      .code
      .ops
      .last()
      .is_some_and(|op| *op == Op::Return);

    if !last_is_return {
      let idx = self.add_const(Object::None);
      self.emit(Op::LoadConst(idx));
      self.emit(Op::Return);
    }

    let func_code = self.scopes.pop().unwrap().code;

    let name = node.name.id.to_string();

    let func = Object::Function {
      name: name.clone(),
      params,
      code: func_code,
    };

    let const_idx = self.add_const(func);

    self.emit(Op::MakeFunction(const_idx));

    let name_idx = self.add_name(&name);

    self.emit(Op::StoreName(name_idx));

    Ok(())
  }

  fn compile_if(&mut self, node: &StmtIf) -> Result<()> {
    self.compile_expr(&node.test)?;

    let false_jump = self.emit_jump(Op::PopJumpIfFalse(0));

    self.compile_body(&node.body)?;

    if node.elif_else_clauses.is_empty() {
      self.patch_jump(false_jump);
      return Ok(());
    }

    let mut end_jumps = vec![];

    for clause in &node.elif_else_clauses {
      end_jumps.push(self.emit_jump(Op::Jump(0)));

      self.patch_jump(false_jump);

      match &clause.test {
        Some(test) => {
          self.compile_expr(test)?;
          let next_false = self.emit_jump(Op::PopJumpIfFalse(0));
          self.compile_body(&clause.body)?;
          end_jumps.push(self.emit_jump(Op::Jump(0)));
          self.patch_jump(next_false);
        }
        None => {
          self.compile_body(&clause.body)?;
        }
      }
    }

    for jump in end_jumps {
      self.patch_jump(jump);
    }

    Ok(())
  }

  fn compile_load_target(&mut self, target: &Expr) -> Result<()> {
    match target {
      Expr::Name(name) => {
        let op = self.resolve_load(&name.id);
        self.emit(op);
        Ok(())
      }
      _ => Err(Error::UnsupportedSyntax {
        message: "complex assignment target".into(),
      }),
    }
  }

  fn compile_number(&mut self, node: &ExprNumberLiteral) -> Result<()> {
    let obj = match &node.value {
      Number::Int(int) => {
        Object::Int(int.as_i64().ok_or_else(|| Error::Compile {
          message: "integer too large".into(),
        })?)
      }
      Number::Float(f) => Object::Float(*f),
      Number::Complex { .. } => {
        return Err(Error::UnsupportedSyntax {
          message: "complex numbers".into(),
        });
      }
    };

    let idx = self.add_const(obj);

    self.emit(Op::LoadConst(idx));

    Ok(())
  }

  fn compile_return(&mut self, node: &StmtReturn) -> Result<()> {
    if let Some(expr) = &node.value {
      self.compile_expr(expr)?;
    } else {
      let idx = self.add_const(Object::None);
      self.emit(Op::LoadConst(idx));
    }

    self.emit(Op::Return);

    Ok(())
  }

  fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
    match stmt {
      Stmt::Assign(node) => self.compile_assign(node),
      Stmt::AugAssign(node) => self.compile_aug_assign(node),
      Stmt::Break(_) => Err(Error::UnsupportedSyntax {
        message: "break (not yet implemented)".into(),
      }),
      Stmt::Continue(_) => Err(Error::UnsupportedSyntax {
        message: "continue (not yet implemented)".into(),
      }),
      Stmt::Expr(node) => {
        self.compile_expr(&node.value)?;
        self.emit(Op::Pop);
        Ok(())
      }
      Stmt::FunctionDef(node) => self.compile_function_def(node),
      Stmt::If(node) => self.compile_if(node),
      Stmt::Pass(_) => Ok(()),
      Stmt::Return(node) => self.compile_return(node),
      Stmt::While(node) => self.compile_while(node),
      _ => Err(Error::UnsupportedSyntax {
        message: format!("statement: {}", stmt_name(stmt)),
      }),
    }
  }

  fn compile_store(&mut self, target: &Expr) -> Result<()> {
    match target {
      Expr::Name(name) => {
        let op = self.resolve_store(&name.id);
        self.emit(op);
        Ok(())
      }
      _ => Err(Error::UnsupportedSyntax {
        message: "complex assignment target".into(),
      }),
    }
  }

  fn compile_while(&mut self, node: &StmtWhile) -> Result<()> {
    let loop_start = self.current_offset();
    self.compile_expr(&node.test)?;
    let exit_jump = self.emit_jump(Op::PopJumpIfFalse(0));
    self.compile_body(&node.body)?;
    self.emit(Op::Jump(loop_start));
    self.patch_jump(exit_jump);
    Ok(())
  }

  fn current_offset(&self) -> u16 {
    u16::try_from(self.scope().code.ops.len())
      .expect("instruction offset overflow")
  }

  fn emit(&mut self, op: Op) {
    self.scope_mut().code.ops.push(op);
  }

  fn emit_jump(&mut self, op: Op) -> usize {
    let idx = self.scope().code.ops.len();
    self.emit(op);
    idx
  }

  fn patch_jump(&mut self, idx: usize) {
    let target =
      u16::try_from(self.scope().code.ops.len()).expect("jump target overflow");

    let op = &mut self.scope_mut().code.ops[idx];

    match op {
      Op::Jump(t) | Op::PopJumpIfFalse(t) | Op::PopJumpIfTrue(t) => *t = target,
      _ => panic!("attempted to patch non-jump instruction"),
    }
  }

  fn resolve_load(&mut self, name: &str) -> Op {
    if self.scope().in_function
      && let Some(idx) = self.scope().code.locals.iter().position(|n| n == name)
    {
      return Op::LoadFast(u16::try_from(idx).expect("local table overflow"));
    }

    let idx = self.add_name(name);

    Op::LoadName(idx)
  }

  fn resolve_store(&mut self, name: &str) -> Op {
    if self.scope().in_function {
      let idx = self.add_local(name);
      Op::StoreFast(idx)
    } else {
      let idx = self.add_name(name);
      Op::StoreName(idx)
    }
  }

  fn scope(&self) -> &Scope {
    self.scopes.last().unwrap()
  }

  fn scope_mut(&mut self) -> &mut Scope {
    self.scopes.last_mut().unwrap()
  }
}

fn expr_name(expr: &Expr) -> &'static str {
  match expr {
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

fn operator_to_binary_op(op: Operator) -> Result<Op> {
  match op {
    Operator::Add => Ok(Op::BinaryAdd),
    Operator::Sub => Ok(Op::BinarySub),
    Operator::Mult => Ok(Op::BinaryMul),
    Operator::Div => Ok(Op::BinaryDiv),
    Operator::FloorDiv => Ok(Op::BinaryFloorDiv),
    Operator::Mod => Ok(Op::BinaryMod),
    Operator::Pow => Ok(Op::BinaryPow),
    _ => Err(Error::UnsupportedSyntax {
      message: format!("operator: {}", op.as_str()),
    }),
  }
}

fn stmt_name(stmt: &Stmt) -> &'static str {
  match stmt {
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

#[cfg(test)]
mod tests {
  use {
    super::*,
    ruff_python_parser::{Mode, parse},
  };

  fn compile(source: &str) -> Code {
    Compiler::compile(
      parse(source, Mode::Module.into())
        .unwrap()
        .try_into_module()
        .unwrap()
        .syntax(),
    )
    .unwrap()
  }

  #[test]
  fn assign_int() {
    let code = compile("foo = 42\n");

    assert_eq!(code.ops, [Op::LoadConst(0), Op::StoreName(0)]);
    assert_eq!(code.constants[0], Object::Int(42));
    assert_eq!(code.names[0], "foo");
  }

  #[test]
  fn aug_assign() {
    assert_eq!(
      compile("foo += 1\n").ops,
      [
        Op::LoadName(0),
        Op::LoadConst(0),
        Op::BinaryAdd,
        Op::StoreName(0),
      ]
    );
  }

  #[test]
  fn binary_add() {
    assert_eq!(
      compile("foo = 1 + 2\n").ops,
      [
        Op::LoadConst(0),
        Op::LoadConst(1),
        Op::BinaryAdd,
        Op::StoreName(0),
      ]
    );
  }

  #[test]
  fn bool_op_and() {
    assert_eq!(
      compile("foo and bar\n").ops,
      [
        Op::LoadName(0),
        Op::Dup,
        Op::PopJumpIfFalse(5),
        Op::Pop,
        Op::LoadName(1),
        Op::Pop,
      ]
    );
  }

  #[test]
  fn comparison() {
    assert_eq!(
      compile("foo < bar\n").ops,
      [Op::LoadName(0), Op::LoadName(1), Op::CompareLt, Op::Pop]
    );
  }

  #[test]
  fn expression_statement() {
    assert_eq!(compile("42\n").ops, [Op::LoadConst(0), Op::Pop]);
  }

  #[test]
  fn function_call() {
    assert_eq!(
      compile("foo(bar, baz)\n").ops,
      [
        Op::LoadName(0),
        Op::LoadName(1),
        Op::LoadName(2),
        Op::CallFunction(2),
        Op::Pop,
      ]
    );
  }

  #[test]
  fn function_def() {
    let code = compile("def foo(bar):\n    return bar\n");

    assert_eq!(code.ops, [Op::MakeFunction(0), Op::StoreName(0)]);

    let func = match &code.constants[0] {
      Object::Function { name, params, code } => {
        assert_eq!(name, "foo");
        assert_eq!(params, &["bar"]);
        code.clone()
      }
      _ => panic!("expected Function"),
    };

    assert_eq!(func.ops, [Op::LoadFast(0), Op::Return]);
    assert_eq!(func.locals, ["bar"]);
  }

  #[test]
  fn if_else() {
    assert_eq!(
      compile("if foo:\n    bar = 1\nelse:\n    bar = 2\n").ops,
      [
        Op::LoadName(0),
        Op::PopJumpIfFalse(5),
        Op::LoadConst(0),
        Op::StoreName(1),
        Op::Jump(7),
        Op::LoadConst(1),
        Op::StoreName(1),
      ]
    );
  }

  #[test]
  fn if_statement() {
    assert_eq!(
      compile("if foo:\n    bar = 1\n").ops,
      [
        Op::LoadName(0),
        Op::PopJumpIfFalse(4),
        Op::LoadConst(0),
        Op::StoreName(1),
      ]
    );
  }

  #[test]
  fn multi_assign() {
    assert_eq!(
      compile("foo = bar = 1\n").ops,
      [
        Op::LoadConst(0),
        Op::Dup,
        Op::StoreName(0),
        Op::StoreName(1)
      ]
    );
  }

  #[test]
  fn ternary() {
    assert_eq!(
      compile("foo if bar else baz\n").ops,
      [
        Op::LoadName(0),
        Op::PopJumpIfFalse(4),
        Op::LoadName(1),
        Op::Jump(5),
        Op::LoadName(2),
        Op::Pop,
      ]
    );
  }

  #[test]
  fn unary_neg() {
    assert_eq!(
      compile("-foo\n").ops,
      [Op::LoadName(0), Op::UnaryNeg, Op::Pop]
    );
  }

  #[test]
  fn while_loop() {
    assert_eq!(
      compile("while foo:\n    bar = 1\n").ops,
      [
        Op::LoadName(0),
        Op::PopJumpIfFalse(5),
        Op::LoadConst(0),
        Op::StoreName(1),
        Op::Jump(0),
      ]
    );
  }
}
