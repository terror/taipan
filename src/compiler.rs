use super::*;

pub struct Compiler {
  scopes: ScopeStack,
}

impl Compiler {
  fn code_mut(&mut self) -> &mut CodeBuilder {
    &mut self.scope_mut().code
  }

  /// Compiles a parsed module into bytecode.
  ///
  /// # Errors
  ///
  /// Returns an error if the module contains unsupported syntax.
  pub fn compile(module: &ModModule) -> Result<Code> {
    let symbols = SymbolTable::module(&module.body)?;

    let mut compiler = Self {
      scopes: ScopeStack::module(symbols),
    };

    compiler.compile_body(&module.body)?;

    compiler.scopes.finish()
  }

  fn compile_assign(&mut self, node: &StmtAssign) -> Result {
    self.compile_expr(&node.value)?;

    for (i, target) in node.targets.iter().enumerate() {
      if i < node.targets.len() - 1 {
        self.code_mut().emit(Instruction::Dup);
      }

      self.compile_store(target)?;
    }

    Ok(())
  }

  fn compile_aug_assign(&mut self, node: &StmtAugAssign) -> Result {
    self.compile_load_target(&node.target)?;
    self.compile_expr(&node.value)?;
    self.code_mut().emit(node.op.instruction()?);
    self.compile_store(&node.target)?;
    Ok(())
  }

  fn compile_body(&mut self, body: &[Stmt]) -> Result {
    for stmt in body {
      self.compile_stmt(stmt)?;
    }

    Ok(())
  }

  fn compile_bool_op(&mut self, node: &ExprBoolOp) -> Result {
    let is_and = matches!(node.op, BoolOp::And);

    let end = self.code_mut().label();

    for (i, value) in node.values.iter().enumerate() {
      self.compile_expr(value)?;

      if i < node.values.len() - 1 {
        self.code_mut().emit(Instruction::Dup);

        if is_and {
          self.code_mut().emit_jump_if_false(end)?;
        } else {
          self.code_mut().emit_jump_if_true(end)?;
        }

        self.code_mut().emit(Instruction::Pop);
      }
    }

    self.code_mut().mark(end)?;

    Ok(())
  }

  fn compile_call(&mut self, node: &ExprCall) -> Result {
    if !node.arguments.keywords.is_empty() {
      return Err(Error::UnsupportedSyntax {
        message: "keyword arguments".into(),
      });
    }

    self.compile_expr(&node.func)?;

    let argc = node.arguments.args.len();

    for argument in &*node.arguments.args {
      self.compile_expr(argument)?;
    }

    let argc = u8::try_from(argc).map_err(|_| Error::Compile {
      message: "too many arguments".into(),
    })?;

    self.code_mut().emit(Instruction::CallFunction(argc));

    Ok(())
  }

  fn compile_compare(&mut self, node: &ExprCompare) -> Result {
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

    self.code_mut().emit(instruction);

    Ok(())
  }

  fn compile_expr(&mut self, expr: &Expr) -> Result {
    match expr {
      Expr::BinOp(node) => {
        self.compile_expr(&node.left)?;
        self.compile_expr(&node.right)?;
        self.code_mut().emit(node.op.instruction()?);
        Ok(())
      }
      Expr::BoolOp(node) => self.compile_bool_op(node),
      Expr::BooleanLiteral(node) => self.emit_const(Object::Bool(node.value)),
      Expr::Call(node) => self.compile_call(node),
      Expr::Compare(node) => self.compile_compare(node),
      Expr::If(node) => {
        let false_label = self.code_mut().label();
        let end = self.code_mut().label();

        self.compile_expr(&node.test)?;
        self.code_mut().emit_jump_if_false(false_label)?;

        self.compile_expr(&node.body)?;
        self.code_mut().emit_jump(end)?;
        self.code_mut().mark(false_label)?;

        self.compile_expr(&node.orelse)?;
        self.code_mut().mark(end)?;

        Ok(())
      }
      Expr::Name(node) => {
        let instruction = self.resolve_load(&node.id)?;
        self.code_mut().emit(instruction);
        Ok(())
      }
      Expr::NoneLiteral(_) => self.emit_none(),
      Expr::NumberLiteral(node) => self.compile_number(node),
      Expr::StringLiteral(node) => {
        self.emit_const(Object::Str(node.value.to_str().to_owned()))
      }
      Expr::UnaryOp(node) => {
        self.compile_expr(&node.operand)?;

        match node.op {
          UnaryOp::USub => self.code_mut().emit(Instruction::UnaryNeg),
          UnaryOp::UAdd => self.code_mut().emit(Instruction::UnaryPos),
          UnaryOp::Not => self.code_mut().emit(Instruction::UnaryNot),
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

  fn compile_function_def(&mut self, node: &StmtFunctionDef) -> Result {
    let parameters = node
      .parameters
      .posonlyargs
      .iter()
      .chain(node.parameters.args.iter())
      .map(|argument| argument.parameter.name.id.to_string())
      .collect::<Vec<_>>();

    let symbols = SymbolTable::function(&node.parameters, &node.body)?;

    self.scopes.enter_function(symbols);

    for local in self.scope().symbols.locals().to_vec() {
      self.code_mut().add_local(&local)?;
    }

    self.compile_body(&node.body)?;

    let last_is_return = self
      .scope()
      .code
      .instructions()
      .last()
      .is_some_and(|instruction| *instruction == Instruction::Return);

    if !last_is_return {
      self.emit_none()?;
      self.code_mut().emit(Instruction::Return);
    }

    let function_code = self.scopes.exit_scope()?;

    let name = node.name.id.to_string();

    let const_index = self.code_mut().add_const(Object::Function {
      name: name.clone(),
      params: parameters,
      code: function_code,
    })?;

    self.code_mut().emit(Instruction::MakeFunction(const_index));

    let instruction = self.resolve_store(&name)?;

    self.code_mut().emit(instruction);

    Ok(())
  }

  fn compile_if(&mut self, node: &StmtIf) -> Result {
    let end = self.code_mut().label();
    let first_clause = self.code_mut().label();

    let mut next_label = Some(first_clause);

    self.compile_expr(&node.test)?;
    self.code_mut().emit_jump_if_false(first_clause)?;

    self.compile_body(&node.body)?;

    for clause in &node.elif_else_clauses {
      self.code_mut().emit_jump(end)?;

      if let Some(label) = next_label {
        self.code_mut().mark(label)?;
      }

      if let Some(test) = &clause.test {
        let label = self.code_mut().label();
        next_label = Some(label);

        self.compile_expr(test)?;
        self.code_mut().emit_jump_if_false(label)?;

        self.compile_body(&clause.body)?;
      } else {
        next_label = None;
        self.compile_body(&clause.body)?;
      }
    }

    if let Some(next_label) = next_label {
      self.code_mut().mark(next_label)?;
    }

    self.code_mut().mark(end)?;

    Ok(())
  }

  fn compile_load_target(&mut self, target: &Expr) -> Result {
    match target {
      Expr::Name(name) => {
        let instruction = self.resolve_load(&name.id)?;
        self.code_mut().emit(instruction);
        Ok(())
      }
      _ => Err(Error::UnsupportedSyntax {
        message: "complex assignment target".into(),
      }),
    }
  }

  fn compile_number(&mut self, node: &ExprNumberLiteral) -> Result {
    self.emit_const(match &node.value {
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
    })
  }

  fn compile_return(&mut self, node: &StmtReturn) -> Result {
    if let Some(expr) = &node.value {
      self.compile_expr(expr)?;
    } else {
      self.emit_none()?;
    }

    self.code_mut().emit(Instruction::Return);

    Ok(())
  }

  fn compile_stmt(&mut self, stmt: &Stmt) -> Result {
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
        self.code_mut().emit(Instruction::Pop);
        Ok(())
      }
      Stmt::FunctionDef(node) => self.compile_function_def(node),
      Stmt::If(node) => self.compile_if(node),
      Stmt::Nonlocal(_) => Err(Error::UnsupportedSyntax {
        message: "nonlocal (not yet implemented)".into(),
      }),
      Stmt::Global(_) | Stmt::Pass(_) => Ok(()),
      Stmt::Return(node) => self.compile_return(node),
      Stmt::While(node) => self.compile_while(node),
      _ => Err(Error::UnsupportedSyntax {
        message: format!("statement: {}", stmt.name()),
      }),
    }
  }

  fn compile_store(&mut self, target: &Expr) -> Result {
    match target {
      Expr::Name(name) => {
        let instruction = self.resolve_store(&name.id)?;
        self.code_mut().emit(instruction);
        Ok(())
      }
      _ => Err(Error::UnsupportedSyntax {
        message: "complex assignment target".into(),
      }),
    }
  }

  fn compile_while(&mut self, node: &StmtWhile) -> Result {
    let start = self.code_mut().label();
    let end = self.code_mut().label();

    self.code_mut().mark(start)?;

    self.compile_expr(&node.test)?;
    self.code_mut().emit_jump_if_false(end)?;

    self.compile_body(&node.body)?;

    self.code_mut().emit_jump(start)?;
    self.code_mut().mark(end)?;

    Ok(())
  }

  fn emit_const(&mut self, object: Object) -> Result {
    let index = self.code_mut().add_const(object)?;

    self.code_mut().emit(Instruction::LoadConst(index));

    Ok(())
  }

  fn emit_none(&mut self) -> Result {
    self.emit_const(Object::None)
  }

  fn resolve_load(&mut self, name: &str) -> Result<Instruction> {
    match self.scope().symbols.resolve(name) {
      Symbol::Local => {
        let index =
          self.scope().symbols.local_index(name).ok_or_else(|| {
            Error::Compile {
              message: format!("missing local: {name}"),
            }
          })?;

        Ok(Instruction::LoadFast(index))
      }
      Symbol::Global | Symbol::Name => {
        Ok(Instruction::LoadName(self.code_mut().add_name(name)?))
      }
      Symbol::Nonlocal => Err(Error::UnsupportedSyntax {
        message: "nonlocal (not yet implemented)".into(),
      }),
    }
  }

  fn resolve_store(&mut self, name: &str) -> Result<Instruction> {
    match self.scope().symbols.resolve(name) {
      Symbol::Global | Symbol::Name => {
        Ok(Instruction::StoreName(self.code_mut().add_name(name)?))
      }
      Symbol::Local => Ok(Instruction::StoreFast(
        self.scope().symbols.local_index(name).ok_or_else(|| {
          Error::Compile {
            message: format!("missing local: {name}"),
          }
        })?,
      )),
      Symbol::Nonlocal => Err(Error::UnsupportedSyntax {
        message: "nonlocal (not yet implemented)".into(),
      }),
    }
  }

  fn scope(&self) -> &Scope {
    self.scopes.current()
  }

  fn scope_mut(&mut self) -> &mut Scope {
    self.scopes.current_mut()
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
  fn function_def_positional_only_parameters() {
    Test::new(indoc! {
      "
      def baz(foo, /, bar):
        return foo - bar
      "
    })
    .instructions(&[Instruction::MakeFunction(0), Instruction::StoreName(0)])
    .constant(Object::Function {
      name: "baz".to_owned(),
      params: vec!["foo".to_owned(), "bar".to_owned()],
      code: Test::default()
        .instructions(&[
          Instruction::LoadFast(0),
          Instruction::LoadFast(1),
          Instruction::BinarySub,
          Instruction::Return,
        ])
        .locals(&["foo", "bar"])
        .code(),
    })
    .names(&["baz"])
    .run();
  }

  #[test]
  fn function_global() {
    Test::new(indoc! {
      "
      def foo():
        global bar
        bar = 1
      "
    })
    .instructions(&[Instruction::MakeFunction(0), Instruction::StoreName(0)])
    .constant(Object::Function {
      name: "foo".to_owned(),
      params: Vec::new(),
      code: Test::default()
        .instructions(&[
          Instruction::LoadConst(0),
          Instruction::StoreName(0),
          Instruction::LoadConst(1),
          Instruction::Return,
        ])
        .constant(Object::Int(1))
        .constant(Object::None)
        .names(&["bar"])
        .code(),
    })
    .names(&["foo"])
    .run();
  }

  #[test]
  fn function_local_from_later_assignment() {
    Test::new(indoc! {
      "
      def foo():
        bar
        bar = 1
      "
    })
    .instructions(&[Instruction::MakeFunction(0), Instruction::StoreName(0)])
    .constant(Object::Function {
      name: "foo".to_owned(),
      params: Vec::new(),
      code: Test::default()
        .instructions(&[
          Instruction::LoadFast(0),
          Instruction::Pop,
          Instruction::LoadConst(0),
          Instruction::StoreFast(0),
          Instruction::LoadConst(1),
          Instruction::Return,
        ])
        .constant(Object::Int(1))
        .constant(Object::None)
        .locals(&["bar"])
        .code(),
    })
    .names(&["foo"])
    .run();
  }

  #[test]
  fn nested_function_def_is_local() {
    Test::new(indoc! {
      "
      def foo():
        def bar():
          return 1
      "
    })
    .instructions(&[Instruction::MakeFunction(0), Instruction::StoreName(0)])
    .constant(Object::Function {
      name: "foo".to_owned(),
      params: Vec::new(),
      code: Test::default()
        .instructions(&[
          Instruction::MakeFunction(0),
          Instruction::StoreFast(0),
          Instruction::LoadConst(1),
          Instruction::Return,
        ])
        .constant(Object::Function {
          name: "bar".to_owned(),
          params: Vec::new(),
          code: Test::default()
            .instructions(&[Instruction::LoadConst(0), Instruction::Return])
            .constant(Object::Int(1))
            .code(),
        })
        .constant(Object::None)
        .locals(&["bar"])
        .code(),
    })
    .names(&["foo"])
    .run();
  }

  #[test]
  fn if_elif_else() {
    Test::new(indoc! {
      "
      if foo:
        bar = 1
      elif baz:
        bar = 2
      else:
        bar = 3
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::PopJumpIfFalse(5),
      Instruction::LoadConst(0),
      Instruction::StoreName(1),
      Instruction::Jump(12),
      Instruction::LoadName(2),
      Instruction::PopJumpIfFalse(10),
      Instruction::LoadConst(1),
      Instruction::StoreName(1),
      Instruction::Jump(12),
      Instruction::LoadConst(2),
      Instruction::StoreName(1),
    ])
    .constant(Object::Int(1))
    .constant(Object::Int(2))
    .constant(Object::Int(3))
    .names(&["foo", "bar", "baz"])
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
