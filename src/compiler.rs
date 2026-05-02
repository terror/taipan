use super::*;

pub struct Compiler {
  scopes: ScopeStack,
}

impl Compiler {
  fn code_mut(&mut self) -> &mut CodeBuilder {
    &mut self.scope_mut().code
  }

  /// Compiles Python source and its parsed module into bytecode.
  ///
  /// # Errors
  ///
  /// Returns an error if the module contains unsupported syntax.
  pub fn compile(source: &str, module: &ModModule) -> Result<Code> {
    Pipeline::with_default_passes(
      Context::builder().module(module).source(source).build(),
    )
    .run()?
    .code()
  }

  fn compile_ann_assign(
    &mut self,
    target: &Expr,
    value: Option<&Expr>,
  ) -> Result {
    if let Some(value) = value {
      self.compile_expr(value)?;
      self.compile_store(target)?;
    }

    Ok(())
  }

  fn compile_assign(&mut self, targets: &[Expr], value: &Expr) -> Result {
    self.compile_expr(value)?;

    for (i, target) in targets.iter().enumerate() {
      if i < targets.len() - 1 {
        self.code_mut().emit(Instruction::Dup);
      }

      self.compile_store(target)?;
    }

    Ok(())
  }

  fn compile_aug_assign(
    &mut self,
    target: &Expr,
    operator: BinaryOperator,
    value: &Expr,
  ) -> Result {
    self.compile_load_target(target)?;

    self.compile_expr(value)?;
    self.code_mut().emit(operator.instruction()?);

    self.compile_store(target)?;

    Ok(())
  }

  pub(crate) fn compile_body(&mut self, body: &[Stmt]) -> Result {
    for stmt in body {
      self.compile_stmt(stmt)?;
    }

    Ok(())
  }

  fn compile_bool_op(
    &mut self,
    operator: BoolOperator,
    values: &[Expr],
  ) -> Result {
    let end = self.code_mut().label();

    for (i, value) in values.iter().enumerate() {
      self.compile_expr(value)?;

      if i < values.len() - 1 {
        self.code_mut().emit(Instruction::Dup);

        match operator {
          BoolOperator::And => self.code_mut().emit_jump_if_false(end)?,
          BoolOperator::Or => self.code_mut().emit_jump_if_true(end)?,
        }

        self.code_mut().emit(Instruction::Pop);
      }
    }

    self.code_mut().mark(end)?;

    Ok(())
  }

  fn compile_break(&mut self) -> Result {
    let control_flow =
      *self
        .scope()
        .control_flows
        .last()
        .ok_or_else(|| Error::Compile {
          message: "break outside loop".into(),
        })?;

    for _ in 0..control_flow.break_stack_pops {
      self.code_mut().emit(Instruction::Pop);
    }

    self.code_mut().emit_jump(control_flow.break_label)
  }

  fn compile_call(&mut self, function: &Expr, arguments: &[Expr]) -> Result {
    self.compile_expr(function)?;

    let argc = arguments.len();

    for argument in arguments {
      self.compile_expr(argument)?;
    }

    let argc = u8::try_from(argc).map_err(|_| Error::Compile {
      message: "too many arguments".into(),
    })?;

    self.code_mut().emit(Instruction::CallFunction(argc));

    Ok(())
  }

  fn compile_compare(
    &mut self,
    lhs: &Expr,
    operator: CompareOperator,
    rhs: &Expr,
  ) -> Result {
    self.compile_expr(lhs)?;
    self.compile_expr(rhs)?;

    let instruction = match operator {
      CompareOperator::Eq => Instruction::CompareEq,
      CompareOperator::Ne => Instruction::CompareNe,
      CompareOperator::Lt => Instruction::CompareLt,
      CompareOperator::Le => Instruction::CompareLe,
      CompareOperator::Gt => Instruction::CompareGt,
      CompareOperator::Ge => Instruction::CompareGe,
      CompareOperator::In => Instruction::CompareIn,
      CompareOperator::NotIn => Instruction::CompareNotIn,
    };

    self.code_mut().emit(instruction);

    Ok(())
  }

  fn compile_continue(&mut self) -> Result {
    let control_flow =
      *self
        .scope()
        .control_flows
        .last()
        .ok_or_else(|| Error::Compile {
          message: "continue outside loop".into(),
        })?;

    self.code_mut().emit_jump(control_flow.continue_label)
  }

  fn compile_expr(&mut self, expr: &Expr) -> Result {
    match expr {
      Expr::Binary { lhs, operator, rhs } => {
        self.compile_expr(lhs)?;
        self.compile_expr(rhs)?;
        self.code_mut().emit(operator.instruction()?);
        Ok(())
      }
      Expr::Bool(value) => self.emit_const(Object::Bool(*value)),
      Expr::BoolOp { operator, values } => {
        self.compile_bool_op(*operator, values)
      }
      Expr::Call {
        arguments,
        function,
      } => self.compile_call(function, arguments),
      Expr::Compare { lhs, operator, rhs } => {
        self.compile_compare(lhs, *operator, rhs)
      }
      Expr::Float(value) => self.emit_const(Object::Float(*value)),
      Expr::FormattedString(expressions) => {
        for expr in expressions {
          self.compile_expr(expr)?;
        }

        let count =
          u16::try_from(expressions.len()).map_err(|_| Error::Compile {
            message: "too many f-string parts".into(),
          })?;

        self.code_mut().emit(Instruction::BuildString(count));

        Ok(())
      }
      Expr::If { body, orelse, test } => {
        let false_label = self.code_mut().label();
        let end = self.code_mut().label();

        self.compile_expr(test)?;
        self.code_mut().emit_jump_if_false(false_label)?;

        self.compile_expr(body)?;
        self.code_mut().emit_jump(end)?;
        self.code_mut().mark(false_label)?;

        self.compile_expr(orelse)?;
        self.code_mut().mark(end)?;

        Ok(())
      }
      Expr::Int(value) => self.emit_const(Object::Int(*value)),
      Expr::List(elements) => {
        for element in elements {
          self.compile_expr(element)?;
        }

        let count =
          u16::try_from(elements.len()).map_err(|_| Error::Compile {
            message: "too many list elements".into(),
          })?;

        self.code_mut().emit(Instruction::BuildList(count));

        Ok(())
      }
      Expr::Name(name) => {
        let instruction = self.scopes.resolve_load(name)?;
        self.code_mut().emit(instruction);
        Ok(())
      }
      Expr::None => self.emit_none(),
      Expr::String(value) => self.emit_const(Object::Str(value.clone())),
      Expr::Subscript { slice, value } => {
        self.compile_expr(value)?;
        self.compile_expr(slice)?;
        self.code_mut().emit(Instruction::BinarySubscript);
        Ok(())
      }
      Expr::Tuple(elements) => {
        for element in elements {
          self.compile_expr(element)?;
        }

        let count =
          u16::try_from(elements.len()).map_err(|_| Error::Compile {
            message: "too many tuple elements".into(),
          })?;

        self.code_mut().emit(Instruction::BuildTuple(count));

        Ok(())
      }
      Expr::Unary { operand, operator } => {
        self.compile_expr(operand)?;

        match operator {
          UnaryOperator::Invert => {
            self.code_mut().emit(Instruction::UnaryInvert);
          }
          UnaryOperator::USub => self.code_mut().emit(Instruction::UnaryNeg),
          UnaryOperator::UAdd => self.code_mut().emit(Instruction::UnaryPos),
          UnaryOperator::Not => self.code_mut().emit(Instruction::UnaryNot),
        }

        Ok(())
      }
    }
  }

  fn compile_for(
    &mut self,
    target: &Expr,
    iter: &Expr,
    body: &[Stmt],
    else_body: &[Stmt],
  ) -> Result {
    let start = self.code_mut().label();
    let orelse = self.code_mut().label();
    let end = self.code_mut().label();

    self.compile_expr(iter)?;
    self.code_mut().emit(Instruction::GetIter);

    self.code_mut().mark(start)?;
    self.code_mut().emit_for_iter(orelse)?;
    self.compile_store(target)?;

    self.compile_loop_body(body, end, start, 1)?;

    self.code_mut().emit_jump(start)?;
    self.code_mut().mark(orelse)?;
    self.compile_body(else_body)?;
    self.code_mut().mark(end)?;

    Ok(())
  }

  fn compile_function_def(&mut self, function: &FunctionDef) -> Result {
    let symbols = SymbolTable::function(function)?;

    let parameters = function
      .parameters
      .iter()
      .map(|parameter| parameter.name.clone())
      .collect::<Vec<_>>();

    let default_count = function
      .parameters
      .iter()
      .filter(|parameter| parameter.default.is_some())
      .count();

    self.scopes.enter_function(symbols);

    for local in self.scope().symbols.locals().to_vec() {
      self.code_mut().add_local(&local)?;
    }

    self.compile_body(&function.body)?;

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

    for freevar in &function_code.freevars {
      self.scopes.ensure_capturable(freevar)?;
    }

    for parameter in function
      .parameters
      .iter()
      .filter_map(|parameter| parameter.default.as_ref())
    {
      self.compile_expr(parameter)?;
    }

    let const_index = self.code_mut().add_const(Object::Function {
      closure: Vec::new(),
      defaults: Vec::new(),
      name: function.name.clone(),
      parameters,
      code: Rc::new(function_code),
    })?;

    let default_count =
      u8::try_from(default_count).map_err(|_| Error::Compile {
        message: "too many default arguments".into(),
      })?;

    self
      .code_mut()
      .emit(Instruction::MakeFunction(const_index, default_count));

    let instruction = self.scopes.resolve_store(&function.name)?;

    self.code_mut().emit(instruction);

    Ok(())
  }

  fn compile_if(
    &mut self,
    test: &Expr,
    body: &[Stmt],
    clauses: &[(Option<Expr>, Vec<Stmt>)],
  ) -> Result {
    let end = self.code_mut().label();
    let first_clause = self.code_mut().label();

    let mut next_label = Some(first_clause);

    self.compile_expr(test)?;
    self.code_mut().emit_jump_if_false(first_clause)?;

    self.compile_body(body)?;

    for (test, body) in clauses {
      self.code_mut().emit_jump(end)?;

      if let Some(label) = next_label {
        self.code_mut().mark(label)?;
      }

      if let Some(test) = test {
        let label = self.code_mut().label();
        next_label = Some(label);

        self.compile_expr(test)?;
        self.code_mut().emit_jump_if_false(label)?;

        self.compile_body(body)?;
      } else {
        next_label = None;
        self.compile_body(body)?;
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
        let instruction = self.scopes.resolve_load(name)?;
        self.code_mut().emit(instruction);
        Ok(())
      }
      Expr::Subscript { slice, value } => {
        self.compile_expr(value)?;
        self.compile_expr(slice)?;
        self.code_mut().emit(Instruction::BinarySubscript);
        Ok(())
      }
      _ => Err(Error::Compile {
        message: "invalid assignment target".into(),
      }),
    }
  }

  fn compile_loop_body(
    &mut self,
    body: &[Stmt],
    break_label: usize,
    continue_label: usize,
    break_stack_pops: u16,
  ) -> Result {
    self.scope_mut().control_flows.push(ControlFlow {
      break_label,
      break_stack_pops,
      continue_label,
    });

    let result = self.compile_body(body);

    self.scope_mut().control_flows.pop();

    result
  }

  fn compile_nonlocal(&mut self, names: &[String]) -> Result {
    for name in names {
      self.scopes.free_index(name)?;
    }

    Ok(())
  }

  fn compile_return(&mut self, value: Option<&Expr>) -> Result {
    if self.scopes.is_module() {
      return Err(Error::Compile {
        message: "'return' outside function".into(),
      });
    }

    if let Some(expr) = value {
      self.compile_expr(expr)?;
    } else {
      self.emit_none()?;
    }

    self.code_mut().emit(Instruction::Return);

    Ok(())
  }

  /// Compiles Python source into bytecode.
  ///
  /// # Errors
  ///
  /// Returns an error if parsing fails or the module contains unsupported
  /// syntax.
  pub fn compile_source(source: &str) -> Result<Code> {
    let module = parse(source, Mode::Module.into())?
      .try_into_module()
      .ok_or_else(|| Error::Internal {
        message: "Mode::Module should produce ModModule".into(),
      })?;

    Self::compile(source, module.syntax())
  }

  fn compile_stmt(&mut self, stmt: &Stmt) -> Result {
    match stmt {
      Stmt::AnnAssign { target, value } => {
        self.compile_ann_assign(target, value.as_ref())
      }
      Stmt::Assign { targets, value } => self.compile_assign(targets, value),
      Stmt::AugAssign {
        operator,
        target,
        value,
      } => self.compile_aug_assign(target, *operator, value),
      Stmt::Break => self.compile_break(),
      Stmt::Continue => self.compile_continue(),
      Stmt::Expr(expr) => {
        self.compile_expr(expr)?;
        self.code_mut().emit(Instruction::Pop);
        Ok(())
      }
      Stmt::For {
        body,
        iter,
        orelse,
        target,
      } => self.compile_for(target, iter, body, orelse),
      Stmt::FunctionDef(function) => self.compile_function_def(function),
      Stmt::Global(_) | Stmt::Pass => Ok(()),
      Stmt::If {
        body,
        clauses,
        test,
      } => self.compile_if(test, body, clauses),
      Stmt::Nonlocal(names) => self.compile_nonlocal(names),
      Stmt::Return(value) => self.compile_return(value.as_ref()),
      Stmt::While { body, orelse, test } => {
        self.compile_while(test, body, orelse)
      }
    }
  }

  fn compile_store(&mut self, target: &Expr) -> Result {
    match target {
      Expr::List(elements) | Expr::Tuple(elements) => {
        let count =
          u16::try_from(elements.len()).map_err(|_| Error::Compile {
            message: "too many assignment targets".into(),
          })?;

        self.code_mut().emit(Instruction::UnpackSequence(count));

        for element in elements {
          self.compile_store(element)?;
        }

        Ok(())
      }
      Expr::Name(name) => {
        let instruction = self.scopes.resolve_store(name)?;
        self.code_mut().emit(instruction);
        Ok(())
      }
      Expr::Subscript { slice, value } => {
        self.compile_expr(value)?;
        self.compile_expr(slice)?;
        self.code_mut().emit(Instruction::StoreSubscript);
        Ok(())
      }
      _ => Err(Error::Compile {
        message: "invalid assignment target".into(),
      }),
    }
  }

  fn compile_while(
    &mut self,
    test: &Expr,
    body: &[Stmt],
    else_body: &[Stmt],
  ) -> Result {
    let start = self.code_mut().label();
    let orelse = self.code_mut().label();
    let end = self.code_mut().label();

    self.code_mut().mark(start)?;

    self.compile_expr(test)?;
    self.code_mut().emit_jump_if_false(orelse)?;

    self.compile_loop_body(body, end, start, 0)?;

    self.code_mut().emit_jump(start)?;
    self.code_mut().mark(orelse)?;
    self.compile_body(else_body)?;
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

  pub(crate) fn finish(self) -> Result<Code> {
    self.scopes.finish()
  }

  pub(crate) fn new(symbols: SymbolTable) -> Self {
    Self {
      scopes: ScopeStack::module(symbols),
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
    freevars: Vec<&'static str>,
    instructions: Vec<Instruction>,
    locals: Vec<&'static str>,
    names: Vec<&'static str>,
    source: &'static str,
  }

  impl Test {
    fn code(self) -> Code {
      Code {
        constants: self.constants,
        freevars: self.freevars.into_iter().map(str::to_owned).collect(),
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

    fn function_code(self) -> Rc<Code> {
      Rc::new(self.code())
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

      assert_eq!(
        Compiler::compile(self.source, module.syntax()).unwrap(),
        self.code()
      );
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
  fn annotated_assign_initialized() {
    Test::new(indoc! {
      "
      foo: int = 1
      "
    })
    .instructions(&[Instruction::LoadConst(0), Instruction::StoreName(0)])
    .constant(Object::Int(1))
    .names(&["foo"])
    .run();
  }

  #[test]
  fn annotated_assign_uninitialized() {
    Test::new(indoc! {
      "
      foo: int
      "
    })
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
  fn membership_comparison() {
    Test::new(indoc! {
      "
      foo in bar
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::LoadName(1),
      Instruction::CompareIn,
      Instruction::Pop,
    ])
    .names(&["foo", "bar"])
    .run();
  }

  #[test]
  fn not_membership_comparison() {
    Test::new(indoc! {
      "
      foo not in bar
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::LoadName(1),
      Instruction::CompareNotIn,
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
  fn f_string() {
    Test::new(indoc! {
      r#"
      f"foo {bar} {1 + 2}"
      "#
    })
    .instructions(&[
      Instruction::LoadConst(0),
      Instruction::LoadName(0),
      Instruction::LoadConst(1),
      Instruction::LoadConst(2),
      Instruction::LoadConst(3),
      Instruction::BinaryAdd,
      Instruction::BuildString(4),
      Instruction::Pop,
    ])
    .constant(Object::Str("foo ".to_owned()))
    .constant(Object::Str(" ".to_owned()))
    .constant(Object::Int(1))
    .constant(Object::Int(2))
    .names(&["bar"])
    .run();
  }

  #[test]
  fn for_break_else() {
    Test::new(indoc! {
      "
      for foo in bar:
        break
      else:
        baz = 1
      qux = 2
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::GetIter,
      Instruction::ForIter(7),
      Instruction::StoreName(1),
      Instruction::Pop,
      Instruction::Jump(9),
      Instruction::Jump(2),
      Instruction::LoadConst(0),
      Instruction::StoreName(2),
      Instruction::LoadConst(1),
      Instruction::StoreName(3),
    ])
    .constant(Object::Int(1))
    .constant(Object::Int(2))
    .names(&["bar", "foo", "baz", "qux"])
    .run();
  }

  #[test]
  fn list_literal() {
    Test::new(indoc! {
      "
      [foo, 1]
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::LoadConst(0),
      Instruction::BuildList(2),
      Instruction::Pop,
    ])
    .constant(Object::Int(1))
    .names(&["foo"])
    .run();
  }

  #[test]
  fn tuple_literal() {
    Test::new(indoc! {
      "
      (foo, 1)
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::LoadConst(0),
      Instruction::BuildTuple(2),
      Instruction::Pop,
    ])
    .constant(Object::Int(1))
    .names(&["foo"])
    .run();
  }

  #[test]
  fn unpack_sequence() {
    Test::new(indoc! {
      "
      foo, bar = baz
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::UnpackSequence(2),
      Instruction::StoreName(1),
      Instruction::StoreName(2),
    ])
    .names(&["baz", "foo", "bar"])
    .run();
  }

  #[test]
  fn subscript() {
    Test::new(indoc! {
      "
      foo[0]
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::LoadConst(0),
      Instruction::BinarySubscript,
      Instruction::Pop,
    ])
    .constant(Object::Int(0))
    .names(&["foo"])
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
    .instructions(&[Instruction::MakeFunction(0, 0), Instruction::StoreName(0)])
    .constant(Object::Function {
      closure: Vec::new(),
      defaults: Vec::new(),
      name: "foo".to_owned(),
      parameters: vec!["bar".to_owned()],
      code: Test::default()
        .instructions(&[Instruction::LoadFast(0), Instruction::Return])
        .locals(&["bar"])
        .function_code(),
    })
    .names(&["foo"])
    .run();
  }

  #[test]
  fn function_def_default_argument() {
    Test::new(indoc! {
      "
      def foo(bar=1):
        return bar
      "
    })
    .instructions(&[
      Instruction::LoadConst(0),
      Instruction::MakeFunction(1, 1),
      Instruction::StoreName(0),
    ])
    .constant(Object::Int(1))
    .constant(Object::Function {
      closure: Vec::new(),
      defaults: Vec::new(),
      name: "foo".to_owned(),
      parameters: vec!["bar".to_owned()],
      code: Test::default()
        .instructions(&[Instruction::LoadFast(0), Instruction::Return])
        .locals(&["bar"])
        .function_code(),
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
    .instructions(&[Instruction::MakeFunction(0, 0), Instruction::StoreName(0)])
    .constant(Object::Function {
      closure: Vec::new(),
      defaults: Vec::new(),
      name: "baz".to_owned(),
      parameters: vec!["foo".to_owned(), "bar".to_owned()],
      code: Test::default()
        .instructions(&[
          Instruction::LoadFast(0),
          Instruction::LoadFast(1),
          Instruction::BinarySub,
          Instruction::Return,
        ])
        .locals(&["foo", "bar"])
        .function_code(),
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
    .instructions(&[Instruction::MakeFunction(0, 0), Instruction::StoreName(0)])
    .constant(Object::Function {
      closure: Vec::new(),
      defaults: Vec::new(),
      name: "foo".to_owned(),
      parameters: Vec::new(),
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
        .function_code(),
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
    .instructions(&[Instruction::MakeFunction(0, 0), Instruction::StoreName(0)])
    .constant(Object::Function {
      closure: Vec::new(),
      defaults: Vec::new(),
      name: "foo".to_owned(),
      parameters: Vec::new(),
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
        .function_code(),
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
  fn nested_function_def_is_local() {
    Test::new(indoc! {
      "
      def foo():
        def bar():
          return 1
      "
    })
    .instructions(&[Instruction::MakeFunction(0, 0), Instruction::StoreName(0)])
    .constant(Object::Function {
      closure: Vec::new(),
      defaults: Vec::new(),
      name: "foo".to_owned(),
      parameters: Vec::new(),
      code: Test::default()
        .instructions(&[
          Instruction::MakeFunction(0, 0),
          Instruction::StoreFast(0),
          Instruction::LoadConst(1),
          Instruction::Return,
        ])
        .constant(Object::Function {
          closure: Vec::new(),
          defaults: Vec::new(),
          name: "bar".to_owned(),
          parameters: Vec::new(),
          code: Test::default()
            .instructions(&[Instruction::LoadConst(0), Instruction::Return])
            .constant(Object::Int(1))
            .function_code(),
        })
        .constant(Object::None)
        .locals(&["bar"])
        .function_code(),
    })
    .names(&["foo"])
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
  fn while_else() {
    Test::new(indoc! {
      "
      while foo:
        bar = 1
      else:
        baz = 2
      "
    })
    .instructions(&[
      Instruction::LoadName(0),
      Instruction::PopJumpIfFalse(5),
      Instruction::LoadConst(0),
      Instruction::StoreName(1),
      Instruction::Jump(0),
      Instruction::LoadConst(1),
      Instruction::StoreName(2),
    ])
    .constant(Object::Int(1))
    .constant(Object::Int(2))
    .names(&["foo", "bar", "baz"])
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
