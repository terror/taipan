use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ScopeKind {
  Function,
  #[default]
  Module,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Symbol {
  Global,
  Local,
  Name,
  Nonlocal,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SymbolTable {
  bindings: HashSet<String>,
  globals: HashSet<String>,
  kind: ScopeKind,
  local_indices: HashMap<String, u16>,
  locals: Vec<String>,
  nonlocals: HashSet<String>,
  uses: HashSet<String>,
}

impl SymbolTable {
  fn analyze_arguments(
    &mut self,
    arguments: &ruff_python_ast::Arguments,
  ) -> Result {
    for argument in &*arguments.args {
      self.analyze_expr(argument)?;
    }

    for keyword in &*arguments.keywords {
      self.analyze_expr(&keyword.value)?;
    }

    Ok(())
  }

  fn analyze_body(&mut self, body: &[Stmt]) -> Result {
    for stmt in body {
      self.analyze_stmt(stmt)?;
    }

    Ok(())
  }

  fn analyze_class_def(
    &mut self,
    node: &ruff_python_ast::StmtClassDef,
  ) -> Result {
    for decorator in &node.decorator_list {
      self.analyze_expr(&decorator.expression)?;
    }

    if let Some(arguments) = &node.arguments {
      self.analyze_arguments(arguments)?;
    }

    self.bind_local(&node.name.id)
  }

  fn analyze_comprehensions(
    &mut self,
    comprehensions: &[ruff_python_ast::Comprehension],
  ) -> Result {
    for comprehension in comprehensions {
      self.analyze_expr(&comprehension.iter)?;
      self.bind_target(&comprehension.target)?;

      for condition in &comprehension.ifs {
        self.analyze_expr(condition)?;
      }
    }

    Ok(())
  }

  fn analyze_elements(&mut self, elements: &[Expr]) -> Result {
    for element in elements {
      self.analyze_expr(element)?;
    }

    Ok(())
  }

  fn analyze_except_handler(&mut self, handler: &ExceptHandler) -> Result {
    let ExceptHandler::ExceptHandler(handler) = handler;

    if let Some(type_) = &handler.type_ {
      self.analyze_expr(type_)?;
    }

    if let Some(name) = &handler.name {
      self.bind_local(&name.id)?;
    }

    self.analyze_body(&handler.body)
  }

  fn analyze_expr(&mut self, expr: &Expr) -> Result {
    match expr {
      Expr::Attribute(node) => self.analyze_expr(&node.value)?,
      Expr::Await(node) => self.analyze_expr(&node.value)?,
      Expr::BinOp(node) => {
        self.analyze_expr(&node.left)?;
        self.analyze_expr(&node.right)?;
      }
      Expr::BoolOp(node) => self.analyze_elements(&node.values)?,
      Expr::Call(node) => {
        self.analyze_expr(&node.func)?;
        self.analyze_arguments(&node.arguments)?;
      }
      Expr::Compare(node) => {
        self.analyze_expr(&node.left)?;
        self.analyze_elements(&node.comparators)?;
      }
      Expr::Dict(node) => {
        for item in &node.items {
          if let Some(key) = &item.key {
            self.analyze_expr(key)?;
          }

          self.analyze_expr(&item.value)?;
        }
      }
      Expr::DictComp(node) => {
        self.analyze_expr(&node.key)?;
        self.analyze_expr(&node.value)?;
        self.analyze_comprehensions(&node.generators)?;
      }
      Expr::Generator(node) => {
        self.analyze_expr(&node.elt)?;
        self.analyze_comprehensions(&node.generators)?;
      }
      Expr::If(node) => {
        self.analyze_expr(&node.test)?;
        self.analyze_expr(&node.body)?;
        self.analyze_expr(&node.orelse)?;
      }
      Expr::List(node) => self.analyze_elements(&node.elts)?,
      Expr::ListComp(node) => {
        self.analyze_expr(&node.elt)?;
        self.analyze_comprehensions(&node.generators)?;
      }
      Expr::Name(node) => self.bind_use(&node.id),
      Expr::Named(node) => {
        self.analyze_expr(&node.value)?;
        self.bind_target(&node.target)?;
      }
      Expr::Set(node) => self.analyze_elements(&node.elts)?,
      Expr::SetComp(node) => {
        self.analyze_expr(&node.elt)?;
        self.analyze_comprehensions(&node.generators)?;
      }
      Expr::Slice(node) => {
        for expr in [&node.lower, &node.upper, &node.step].into_iter().flatten()
        {
          self.analyze_expr(expr)?;
        }
      }
      Expr::Starred(node) => self.analyze_expr(&node.value)?,
      Expr::UnaryOp(node) => self.analyze_expr(&node.operand)?,
      Expr::Subscript(node) => {
        self.analyze_expr(&node.value)?;
        self.analyze_expr(&node.slice)?;
      }
      Expr::Tuple(node) => self.analyze_elements(&node.elts)?,
      Expr::Yield(node) => {
        if let Some(value) = &node.value {
          self.analyze_expr(value)?;
        }
      }
      Expr::YieldFrom(node) => self.analyze_expr(&node.value)?,
      Expr::BooleanLiteral(_)
      | Expr::BytesLiteral(_)
      | Expr::EllipsisLiteral(_)
      | Expr::FString(_)
      | Expr::IpyEscapeCommand(_)
      | Expr::Lambda(_)
      | Expr::NoneLiteral(_)
      | Expr::NumberLiteral(_)
      | Expr::StringLiteral(_)
      | Expr::TString(_) => {}
    }

    Ok(())
  }

  fn analyze_for(&mut self, node: &ruff_python_ast::StmtFor) -> Result {
    self.analyze_expr(&node.iter)?;
    self.bind_target(&node.target)?;
    self.analyze_body(&node.body)?;
    self.analyze_body(&node.orelse)
  }

  fn analyze_function_header(&mut self, node: &StmtFunctionDef) -> Result {
    for decorator in &node.decorator_list {
      self.analyze_expr(&decorator.expression)?;
    }

    self.analyze_parameter_expressions(&node.parameters)
  }

  fn analyze_if(&mut self, node: &StmtIf) -> Result {
    self.analyze_expr(&node.test)?;
    self.analyze_body(&node.body)?;

    for clause in &node.elif_else_clauses {
      if let Some(test) = &clause.test {
        self.analyze_expr(test)?;
      }

      self.analyze_body(&clause.body)?;
    }

    Ok(())
  }

  fn analyze_parameter_expressions(
    &mut self,
    parameters: &Parameters,
  ) -> Result {
    for parameter in parameters.iter_non_variadic_params() {
      if let Some(default) = parameter.default() {
        self.analyze_expr(default)?;
      }

      if let Some(annotation) = parameter.annotation() {
        self.analyze_expr(annotation)?;
      }
    }

    if let Some(vararg) = parameters.vararg.as_deref()
      && let Some(annotation) = vararg.annotation()
    {
      self.analyze_expr(annotation)?;
    }

    if let Some(kwarg) = parameters.kwarg.as_deref()
      && let Some(annotation) = kwarg.annotation()
    {
      self.analyze_expr(annotation)?;
    }

    Ok(())
  }

  fn analyze_stmt(&mut self, stmt: &Stmt) -> Result {
    match stmt {
      Stmt::AnnAssign(node) => {
        self.bind_target(&node.target)?;
        self.analyze_expr(&node.annotation)?;

        if let Some(value) = &node.value {
          self.analyze_expr(value)?;
        }
      }
      Stmt::Assign(node) => {
        self.analyze_expr(&node.value)?;

        for target in &node.targets {
          self.bind_target(target)?;
        }
      }
      Stmt::AugAssign(node) => {
        self.bind_target(&node.target)?;
        self.analyze_expr(&node.value)?;
      }
      Stmt::Assert(node) => {
        self.analyze_expr(&node.test)?;

        if let Some(msg) = &node.msg {
          self.analyze_expr(msg)?;
        }
      }
      Stmt::ClassDef(node) => self.analyze_class_def(node)?,
      Stmt::Delete(node) => {
        for target in &node.targets {
          self.bind_target(target)?;
        }
      }
      Stmt::Expr(node) => self.analyze_expr(&node.value)?,
      Stmt::For(node) => self.analyze_for(node)?,
      Stmt::FunctionDef(node) => {
        self.analyze_function_header(node)?;
        self.bind_local(&node.name.id)?;
      }
      Stmt::Global(node) => {
        for name in &node.names {
          self.bind_global(&name.id)?;
        }
      }
      Stmt::If(node) => self.analyze_if(node)?,
      Stmt::Import(node) => {
        for alias in &node.names {
          self.bind_import(alias)?;
        }
      }
      Stmt::ImportFrom(node) => {
        for alias in &node.names {
          self.bind_import_from(alias)?;
        }
      }
      Stmt::Nonlocal(node) => {
        for name in &node.names {
          self.bind_nonlocal(&name.id)?;
        }
      }
      Stmt::Raise(node) => {
        if let Some(exc) = &node.exc {
          self.analyze_expr(exc)?;
        }

        if let Some(cause) = &node.cause {
          self.analyze_expr(cause)?;
        }
      }
      Stmt::Return(node) => {
        if let Some(value) = &node.value {
          self.analyze_expr(value)?;
        }
      }
      Stmt::Try(node) => self.analyze_try(node)?,
      Stmt::TypeAlias(node) => {
        self.bind_target(&node.name)?;
        self.analyze_expr(&node.value)?;
      }
      Stmt::While(node) => self.analyze_while(node)?,
      Stmt::With(node) => self.analyze_with(node)?,
      Stmt::Break(_)
      | Stmt::Continue(_)
      | Stmt::IpyEscapeCommand(_)
      | Stmt::Match(_)
      | Stmt::Pass(_) => {}
    }

    Ok(())
  }

  fn analyze_try(&mut self, node: &ruff_python_ast::StmtTry) -> Result {
    self.analyze_body(&node.body)?;

    for handler in &node.handlers {
      self.analyze_except_handler(handler)?;
    }

    self.analyze_body(&node.orelse)?;
    self.analyze_body(&node.finalbody)
  }

  fn analyze_while(&mut self, node: &StmtWhile) -> Result {
    self.analyze_expr(&node.test)?;
    self.analyze_body(&node.body)?;
    self.analyze_body(&node.orelse)
  }

  fn analyze_with(&mut self, node: &ruff_python_ast::StmtWith) -> Result {
    for item in &node.items {
      self.analyze_expr(&item.context_expr)?;

      if let Some(optional_vars) = &item.optional_vars {
        self.bind_target(optional_vars)?;
      }
    }

    self.analyze_body(&node.body)
  }

  fn bind_global(&mut self, name: &str) -> Result {
    if self.nonlocals.contains(name) {
      return Err(Error::Compile {
        message: format!("name '{name}' is nonlocal and global"),
      });
    }

    if self.bindings.contains(name) {
      return Err(Error::Compile {
        message: format!(
          "name '{name}' is assigned to before global declaration"
        ),
      });
    }

    if self.uses.contains(name) {
      return Err(Error::Compile {
        message: format!("name '{name}' is used before global declaration"),
      });
    }

    self.globals.insert(name.to_owned());

    Ok(())
  }

  fn bind_import(&mut self, alias: &Alias) -> Result {
    if let Some(name) = &alias.asname {
      return self.bind_local(&name.id);
    }

    if let Some(name) = alias.name.id.split('.').next() {
      self.bind_local(name)?;
    }

    Ok(())
  }

  fn bind_import_from(&mut self, alias: &Alias) -> Result {
    if alias.name.id == "*" {
      return Ok(());
    }

    let name = alias.asname.as_ref().unwrap_or(&alias.name);

    self.bind_local(&name.id)
  }

  fn bind_local(&mut self, name: &str) -> Result {
    if self.globals.contains(name) || self.nonlocals.contains(name) {
      return Ok(());
    }

    self.bindings.insert(name.to_owned());

    if self.kind == ScopeKind::Module {
      return Ok(());
    }

    self.bind_local_index(name)
  }

  fn bind_local_index(&mut self, name: &str) -> Result {
    if self.local_indices.contains_key(name) {
      return Ok(());
    }

    let index =
      u16::try_from(self.locals.len()).map_err(|_| Error::Compile {
        message: "local table overflow".into(),
      })?;

    self.locals.push(name.to_owned());
    self.local_indices.insert(name.to_owned(), index);

    Ok(())
  }

  fn bind_nonlocal(&mut self, name: &str) -> Result {
    if self.kind == ScopeKind::Module {
      return Err(Error::Compile {
        message: format!(
          "nonlocal declaration not allowed at module level: {name}"
        ),
      });
    }

    if self.globals.contains(name) {
      return Err(Error::Compile {
        message: format!("name '{name}' is nonlocal and global"),
      });
    }

    if self.bindings.contains(name) {
      return Err(Error::Compile {
        message: format!(
          "name '{name}' is assigned to before nonlocal declaration"
        ),
      });
    }

    if self.uses.contains(name) {
      return Err(Error::Compile {
        message: format!("name '{name}' is used before nonlocal declaration"),
      });
    }

    self.nonlocals.insert(name.to_owned());

    Ok(())
  }

  fn bind_parameter(&mut self, name: &str) -> Result {
    if self.local_indices.contains_key(name) || self.bindings.contains(name) {
      return Err(Error::Compile {
        message: format!("duplicate parameter: {name}"),
      });
    }

    self.bind_local(name)
  }

  fn bind_parameters(&mut self, parameters: &Parameters) -> Result {
    for parameter in parameters {
      self.bind_parameter(parameter.name().as_str())?;
    }

    Ok(())
  }

  fn bind_target(&mut self, target: &Expr) -> Result {
    match target {
      Expr::Attribute(node) => self.analyze_expr(&node.value)?,
      Expr::List(node) => {
        for element in &node.elts {
          self.bind_target(element)?;
        }
      }
      Expr::Tuple(node) => {
        for element in &node.elts {
          self.bind_target(element)?;
        }
      }
      Expr::Name(name) => self.bind_local(&name.id)?,
      Expr::Starred(node) => self.bind_target(&node.value)?,
      Expr::Subscript(node) => {
        self.analyze_expr(&node.value)?;
        self.analyze_expr(&node.slice)?;
      }
      _ => {}
    }

    Ok(())
  }

  fn bind_use(&mut self, name: &str) {
    self.uses.insert(name.to_owned());
  }

  pub(crate) fn function(
    parameters: &Parameters,
    body: &[Stmt],
  ) -> Result<Self> {
    let mut table = Self {
      kind: ScopeKind::Function,
      ..Self::default()
    };

    table.bind_parameters(parameters)?;

    table.analyze_body(body)?;

    Ok(table)
  }

  pub(crate) fn local_index(&self, name: &str) -> Option<u16> {
    self.local_indices.get(name).copied()
  }

  pub(crate) fn locals(&self) -> &[String] {
    &self.locals
  }

  pub(crate) fn module(body: &[Stmt]) -> Result<Self> {
    let mut table = Self {
      kind: ScopeKind::Module,
      ..Self::default()
    };

    table.analyze_body(body)?;

    Ok(table)
  }

  pub(crate) fn resolve(&self, name: &str) -> Symbol {
    if self.globals.contains(name) {
      Symbol::Global
    } else if self.nonlocals.contains(name) {
      Symbol::Nonlocal
    } else if self.local_indices.contains_key(name) {
      Symbol::Local
    } else {
      Symbol::Name
    }
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

  fn module(source: &str) -> ModModule {
    parse(source, Mode::Module.into())
      .unwrap()
      .try_into_module()
      .unwrap()
      .into_syntax()
  }

  fn function(source: &str) -> StmtFunctionDef {
    let module = module(source);

    let Stmt::FunctionDef(function) = module.body.into_iter().next().unwrap()
    else {
      panic!("expected function definition");
    };

    function
  }

  #[test]
  fn conflicting_global_declaration() {
    let function = function(indoc! {
      "
      def foo():
        bar = 1
        global bar
      "
    });

    assert_eq!(
      SymbolTable::function(&function.parameters, &function.body)
        .unwrap_err()
        .to_string(),
      "CompileError: name 'bar' is assigned to before global declaration",
    );
  }

  #[test]
  fn function_header_does_not_bind_parameters_in_enclosing_scope() {
    let function = function(indoc! {
      "
      def foo():
        def bar(baz):
          return baz
      "
    });

    let table =
      SymbolTable::function(&function.parameters, &function.body).unwrap();

    assert_eq!(table.locals(), &["bar"]);
  }

  #[test]
  fn imports_bind_locals() {
    let function = function(indoc! {
      "
      def foo():
        import bar.baz
        import qux as quux
        from quuz import corge
      "
    });

    let table =
      SymbolTable::function(&function.parameters, &function.body).unwrap();

    assert_eq!(table.locals(), &["bar", "quux", "corge"]);
  }

  #[test]
  fn module_assignment_resolves_as_name() {
    let module = module("foo = 1");
    let table = SymbolTable::module(&module.body).unwrap();

    assert_eq!(table.resolve("foo"), Symbol::Name);
    assert_eq!(table.locals(), &[] as &[String]);
  }

  #[test]
  fn tuple_assignment_binds_locals() {
    let function = function(indoc! {
      "
      def foo():
        bar, baz = qux
      "
    });

    let table =
      SymbolTable::function(&function.parameters, &function.body).unwrap();

    assert_eq!(table.locals(), &["bar", "baz"]);
    assert_eq!(table.resolve("bar"), Symbol::Local);
    assert_eq!(table.resolve("qux"), Symbol::Name);
  }
}
