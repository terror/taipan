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

  /// Compiles a parsed module into bytecode.
  ///
  /// # Errors
  ///
  /// Returns an error if the module contains unsupported syntax.
  pub fn compile(module: &ModModule) -> Result<Code> {
    let mut compiler = Self {
      scopes: vec![Scope {
        code: Code::default(),
        in_function: false,
      }],
    };

    compiler.compile_body(&module.body)?;

    compiler
      .scopes
      .pop()
      .map(|scope| scope.code)
      .ok_or_else(|| Error::Compile {
        message: "missing compiler scope".into(),
      })
  }

  fn compile_assign(&mut self, node: &StmtAssign) -> Result<()> {
    self.compile_expr(&node.value)?;

    for (i, target) in node.targets.iter().enumerate() {
      if i < node.targets.len() - 1 {
        self.emit(Instruction::Dup);
      }

      self.compile_store(target)?;
    }

    Ok(())
  }

  fn compile_aug_assign(&mut self, node: &StmtAugAssign) -> Result<()> {
    self.compile_load_target(&node.target)?;
    self.compile_expr(&node.value)?;
    self.emit(node.op.instruction()?);
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
        self.emit(Instruction::Dup);

        let jump = if is_and {
          self.emit_jump(Instruction::PopJumpIfFalse(0))
        } else {
          self.emit_jump(Instruction::PopJumpIfTrue(0))
        };

        self.emit(Instruction::Pop);

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

    self.emit(Instruction::CallFunction(argc));

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

    let instruction = match node.ops[0] {
      CmpOp::Eq => Instruction::CompareEq,
      CmpOp::NotEq => Instruction::CompareNe,
      CmpOp::Lt => Instruction::CompareLt,
      CmpOp::LtE => Instruction::CompareLe,
      CmpOp::Gt => Instruction::CompareGt,
      CmpOp::GtE => Instruction::CompareGe,
      CmpOp::Is | CmpOp::IsNot | CmpOp::In | CmpOp::NotIn => {
        return Err(Error::UnsupportedSyntax {
          message: format!("comparison operator: {}", node.ops[0].as_str()),
        });
      }
    };

    self.emit(instruction);

    Ok(())
  }

  fn compile_expr(&mut self, expr: &Expr) -> Result<()> {
    match expr {
      Expr::BinOp(node) => {
        self.compile_expr(&node.left)?;
        self.compile_expr(&node.right)?;
        self.emit(node.op.instruction()?);
        Ok(())
      }
      Expr::BoolOp(node) => self.compile_bool_op(node),
      Expr::BooleanLiteral(node) => {
        let idx = self.add_const(Object::Bool(node.value));
        self.emit(Instruction::LoadConst(idx));
        Ok(())
      }
      Expr::Call(node) => self.compile_call(node),
      Expr::Compare(node) => self.compile_compare(node),
      Expr::If(node) => {
        self.compile_expr(&node.test)?;
        let false_jump = self.emit_jump(Instruction::PopJumpIfFalse(0));
        self.compile_expr(&node.body)?;
        let end_jump = self.emit_jump(Instruction::Jump(0));
        self.patch_jump(false_jump);
        self.compile_expr(&node.orelse)?;
        self.patch_jump(end_jump);
        Ok(())
      }
      Expr::Name(node) => {
        let instruction = self.resolve_load(&node.id);
        self.emit(instruction);
        Ok(())
      }
      Expr::NoneLiteral(_) => {
        let idx = self.add_const(Object::None);
        self.emit(Instruction::LoadConst(idx));
        Ok(())
      }
      Expr::NumberLiteral(node) => self.compile_number(node),
      Expr::StringLiteral(node) => {
        let s = node.value.to_str().to_owned();
        let idx = self.add_const(Object::Str(s));
        self.emit(Instruction::LoadConst(idx));
        Ok(())
      }
      Expr::UnaryOp(node) => {
        self.compile_expr(&node.operand)?;
        match node.op {
          UnaryOp::USub => self.emit(Instruction::UnaryNeg),
          UnaryOp::UAdd => self.emit(Instruction::UnaryPos),
          UnaryOp::Not => self.emit(Instruction::UnaryNot),
          UnaryOp::Invert => {
            return Err(Error::UnsupportedSyntax {
              message: "bitwise invert (~)".into(),
            });
          }
        }
        Ok(())
      }
      _ => Err(Error::UnsupportedSyntax {
        message: format!("expression: {}", expr.name()),
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
      .instructions
      .last()
      .is_some_and(|instruction| *instruction == Instruction::Return);

    if !last_is_return {
      let idx = self.add_const(Object::None);
      self.emit(Instruction::LoadConst(idx));
      self.emit(Instruction::Return);
    }

    let func_code = self.scopes.pop().unwrap().code;

    let name = node.name.id.to_string();

    let func = Object::Function {
      name: name.clone(),
      params,
      code: func_code,
    };

    let const_idx = self.add_const(func);

    self.emit(Instruction::MakeFunction(const_idx));

    let name_idx = self.add_name(&name);

    self.emit(Instruction::StoreName(name_idx));

    Ok(())
  }

  fn compile_if(&mut self, node: &StmtIf) -> Result<()> {
    self.compile_expr(&node.test)?;

    let false_jump = self.emit_jump(Instruction::PopJumpIfFalse(0));

    self.compile_body(&node.body)?;

    if node.elif_else_clauses.is_empty() {
      self.patch_jump(false_jump);
      return Ok(());
    }

    let mut end_jumps = vec![];

    for clause in &node.elif_else_clauses {
      end_jumps.push(self.emit_jump(Instruction::Jump(0)));

      self.patch_jump(false_jump);

      match &clause.test {
        Some(test) => {
          self.compile_expr(test)?;
          let next_false = self.emit_jump(Instruction::PopJumpIfFalse(0));
          self.compile_body(&clause.body)?;
          end_jumps.push(self.emit_jump(Instruction::Jump(0)));
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
        let instruction = self.resolve_load(&name.id);
        self.emit(instruction);
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

    self.emit(Instruction::LoadConst(idx));

    Ok(())
  }

  fn compile_return(&mut self, node: &StmtReturn) -> Result<()> {
    if let Some(expr) = &node.value {
      self.compile_expr(expr)?;
    } else {
      let idx = self.add_const(Object::None);
      self.emit(Instruction::LoadConst(idx));
    }

    self.emit(Instruction::Return);

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
        self.emit(Instruction::Pop);
        Ok(())
      }
      Stmt::FunctionDef(node) => self.compile_function_def(node),
      Stmt::If(node) => self.compile_if(node),
      Stmt::Pass(_) => Ok(()),
      Stmt::Return(node) => self.compile_return(node),
      Stmt::While(node) => self.compile_while(node),
      _ => Err(Error::UnsupportedSyntax {
        message: format!("statement: {}", stmt.name()),
      }),
    }
  }

  fn compile_store(&mut self, target: &Expr) -> Result<()> {
    match target {
      Expr::Name(name) => {
        let instruction = self.resolve_store(&name.id);
        self.emit(instruction);
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
    let exit_jump = self.emit_jump(Instruction::PopJumpIfFalse(0));
    self.compile_body(&node.body)?;
    self.emit(Instruction::Jump(loop_start));
    self.patch_jump(exit_jump);
    Ok(())
  }

  fn current_offset(&self) -> u16 {
    u16::try_from(self.scope().code.instructions.len())
      .expect("instruction offset overflow")
  }

  fn emit(&mut self, instruction: Instruction) {
    self.scope_mut().code.instructions.push(instruction);
  }

  fn emit_jump(&mut self, instruction: Instruction) -> usize {
    let idx = self.scope().code.instructions.len();
    self.emit(instruction);
    idx
  }

  fn patch_jump(&mut self, idx: usize) {
    let target = u16::try_from(self.scope().code.instructions.len())
      .expect("jump target overflow");

    let instruction = &mut self.scope_mut().code.instructions[idx];

    match instruction {
      Instruction::Jump(t)
      | Instruction::PopJumpIfFalse(t)
      | Instruction::PopJumpIfTrue(t) => *t = target,
      _ => panic!("attempted to patch non-jump instruction"),
    }
  }

  fn resolve_load(&mut self, name: &str) -> Instruction {
    if self.scope().in_function
      && let Some(idx) = self.scope().code.locals.iter().position(|n| n == name)
    {
      return Instruction::LoadFast(
        u16::try_from(idx).expect("local table overflow"),
      );
    }

    let idx = self.add_name(name);

    Instruction::LoadName(idx)
  }

  fn resolve_store(&mut self, name: &str) -> Instruction {
    if self.scope().in_function {
      let idx = self.add_local(name);
      Instruction::StoreFast(idx)
    } else {
      let idx = self.add_name(name);
      Instruction::StoreName(idx)
    }
  }

  fn scope(&self) -> &Scope {
    self.scopes.last().unwrap()
  }

  fn scope_mut(&mut self) -> &mut Scope {
    self.scopes.last_mut().unwrap()
  }
}

#[cfg(test)]
mod tests {
  use {
    super::*,
    indoc::indoc,
    pretty_assertions::assert_eq,
    ruff_python_parser::{Mode, parse},
  };

  #[derive(Debug, Default)]
  struct Test {
    constants: Vec<Object>,
    instructions: Vec<Instruction>,
    locals: Vec<&'static str>,
    names: Vec<&'static str>,
    source: &'static str,
  }

  impl Test {
    fn code(self) -> Code {
      Code {
        constants: self.constants,
        instructions: self.instructions,
        locals: self.locals.into_iter().map(str::to_owned).collect(),
        names: self.names.into_iter().map(str::to_owned).collect(),
      }
    }

    fn constant(self, constant: Object) -> Self {
      Self {
        constants: self.constants.into_iter().chain([constant]).collect(),
        ..self
      }
    }

    fn instructions(self, instructions: &[Instruction]) -> Self {
      Self {
        instructions: instructions.to_vec(),
        ..self
      }
    }

    fn locals(self, locals: &[&'static str]) -> Self {
      Self {
        locals: locals.to_vec(),
        ..self
      }
    }

    fn names(self, names: &[&'static str]) -> Self {
      Self {
        names: names.to_vec(),
        ..self
      }
    }

    fn new(source: &'static str) -> Self {
      Self {
        source,
        ..Self::default()
      }
    }

    fn run(self) {
      let module = parse(self.source, Mode::Module.into())
        .unwrap()
        .try_into_module()
        .unwrap();

      assert_eq!(Compiler::compile(module.syntax()).unwrap(), self.code());
    }
  }

  #[test]
  fn assign_int() {
    Test::new(indoc! {
      "
      foo = 42
      "
    })
    .instructions(&[Instruction::LoadConst(0), Instruction::StoreName(0)])
    .constant(Object::Int(42))
    .names(&["foo"])
    .run();
  }

  #[test]
  fn aug_assign() {
    Test::new(indoc! {
      "
      foo += 1
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::LoadConst(0),
      Instruction::BinaryAdd,
      Instruction::StoreName(0),
    ])
    .constant(Object::Int(1))
    .names(&["foo"])
    .run();
  }

  #[test]
  fn binary_add() {
    Test::new(indoc! {
      "
      foo = 1 + 2
      "
    })
    .instructions(&[
      Instruction::LoadConst(0),
      Instruction::LoadConst(1),
      Instruction::BinaryAdd,
      Instruction::StoreName(0),
    ])
    .constant(Object::Int(1))
    .constant(Object::Int(2))
    .names(&["foo"])
    .run();
  }

  #[test]
  fn bool_op_and() {
    Test::new(indoc! {
      "
      foo and bar
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::Dup,
      Instruction::PopJumpIfFalse(5),
      Instruction::Pop,
      Instruction::LoadName(1),
      Instruction::Pop,
    ])
    .names(&["foo", "bar"])
    .run();
  }

  #[test]
  fn comparison() {
    Test::new(indoc! {
      "
      foo < bar
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::LoadName(1),
      Instruction::CompareLt,
      Instruction::Pop,
    ])
    .names(&["foo", "bar"])
    .run();
  }

  #[test]
  fn expression_statement() {
    Test::new(indoc! {
      "
      42
      "
    })
    .instructions(&[Instruction::LoadConst(0), Instruction::Pop])
    .constant(Object::Int(42))
    .run();
  }

  #[test]
  fn function_call() {
    Test::new(indoc! {
      "
      foo(bar, baz)
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::LoadName(1),
      Instruction::LoadName(2),
      Instruction::CallFunction(2),
      Instruction::Pop,
    ])
    .names(&["foo", "bar", "baz"])
    .run();
  }

  #[test]
  fn function_def() {
    Test::new(indoc! {
      "
      def foo(bar):
        return bar
      "
    })
    .instructions(&[Instruction::MakeFunction(0), Instruction::StoreName(0)])
    .constant(Object::Function {
      name: "foo".to_owned(),
      params: vec!["bar".to_owned()],
      code: Test::default()
        .instructions(&[Instruction::LoadFast(0), Instruction::Return])
        .locals(&["bar"])
        .code(),
    })
    .names(&["foo"])
    .run();
  }

  #[test]
  fn if_else() {
    Test::new(indoc! {
      "
      if foo:
        bar = 1
      else:
        bar = 2
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::PopJumpIfFalse(5),
      Instruction::LoadConst(0),
      Instruction::StoreName(1),
      Instruction::Jump(7),
      Instruction::LoadConst(1),
      Instruction::StoreName(1),
    ])
    .constant(Object::Int(1))
    .constant(Object::Int(2))
    .names(&["foo", "bar"])
    .run();
  }

  #[test]
  fn if_statement() {
    Test::new(indoc! {
      "
      if foo:
        bar = 1
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::PopJumpIfFalse(4),
      Instruction::LoadConst(0),
      Instruction::StoreName(1),
    ])
    .constant(Object::Int(1))
    .names(&["foo", "bar"])
    .run();
  }

  #[test]
  fn multi_assign() {
    Test::new(indoc! {
      "
      foo = bar = 1
      "
    })
    .instructions(&[
      Instruction::LoadConst(0),
      Instruction::Dup,
      Instruction::StoreName(0),
      Instruction::StoreName(1),
    ])
    .constant(Object::Int(1))
    .names(&["foo", "bar"])
    .run();
  }

  #[test]
  fn ternary() {
    Test::new(indoc! {
      "
      foo if bar else baz
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::PopJumpIfFalse(4),
      Instruction::LoadName(1),
      Instruction::Jump(5),
      Instruction::LoadName(2),
      Instruction::Pop,
    ])
    .names(&["bar", "foo", "baz"])
    .run();
  }

  #[test]
  fn unary_neg() {
    Test::new(indoc! {
      "
      -foo
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::UnaryNeg,
      Instruction::Pop,
    ])
    .names(&["foo"])
    .run();
  }

  #[test]
  fn while_loop() {
    Test::new(indoc! {
      "
      while foo:
        bar = 1
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::PopJumpIfFalse(5),
      Instruction::LoadConst(0),
      Instruction::StoreName(1),
      Instruction::Jump(0),
    ])
    .constant(Object::Int(1))
    .names(&["foo", "bar"])
    .run();
  }
}
