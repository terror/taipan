use super::*;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
  fn analyze_body(&mut self, body: &[Stmt]) -> Result {
    for stmt in body {
      self.analyze_stmt(stmt)?;
    }

    Ok(())
  }

  fn analyze_expr(&mut self, expr: &Expr) -> Result {
    match expr {
      Expr::BoolOp { values, .. } => {
        for value in values {
          self.analyze_expr(value)?;
        }
      }
      Expr::Call {
        arguments,
        function,
        keywords,
      } => {
        self.analyze_expr(function)?;

        for argument in arguments {
          self.analyze_expr(argument)?;
        }

        for keyword in keywords {
          self.analyze_expr(&keyword.value)?;
        }
      }
      Expr::Binary { lhs, rhs, .. } | Expr::Compare { lhs, rhs, .. } => {
        self.analyze_expr(lhs)?;
        self.analyze_expr(rhs)?;
      }
      Expr::FormattedString(expressions) => {
        for expr in expressions {
          self.analyze_expr(expr)?;
        }
      }
      Expr::If { body, orelse, test } => {
        self.analyze_expr(test)?;
        self.analyze_expr(body)?;
        self.analyze_expr(orelse)?;
      }
      Expr::List(elements) | Expr::Tuple(elements) => {
        for element in elements {
          self.analyze_expr(element)?;
        }
      }
      Expr::Name(name) => self.bind_use(name),
      Expr::Subscript { slice, value } => {
        self.analyze_expr(value)?;
        self.analyze_expr(slice)?;
      }
      Expr::Unary { operand, .. } => self.analyze_expr(operand)?,
      Expr::Bool(_)
      | Expr::Float(_)
      | Expr::Int(_)
      | Expr::None
      | Expr::String(_) => {}
    }

    Ok(())
  }

  fn analyze_function_header(&mut self, function: &FunctionDef) -> Result {
    for parameter in &function.parameters {
      if let Some(default) = &parameter.default {
        self.analyze_expr(default)?;
      }
    }

    self.bind_local(&function.name)
  }

  fn analyze_if(
    &mut self,
    test: &Expr,
    body: &[Stmt],
    clauses: &[(Option<Expr>, Vec<Stmt>)],
  ) -> Result {
    self.analyze_expr(test)?;
    self.analyze_body(body)?;

    for (test, body) in clauses {
      if let Some(test) = test {
        self.analyze_expr(test)?;
      }

      self.analyze_body(body)?;
    }

    Ok(())
  }

  fn analyze_stmt(&mut self, stmt: &Stmt) -> Result {
    match stmt {
      Stmt::AnnAssign { target, value } => {
        self.bind_target(target)?;

        if let Some(value) = value {
          self.analyze_expr(value)?;
        }
      }
      Stmt::Assign { targets, value } => {
        self.analyze_expr(value)?;

        for target in targets {
          self.bind_target(target)?;
        }
      }
      Stmt::AugAssign { target, value, .. } => {
        self.bind_target(target)?;
        self.analyze_expr(value)?;
      }
      Stmt::Expr(expr) => self.analyze_expr(expr)?,
      Stmt::For {
        body,
        iter,
        orelse,
        target,
      } => {
        self.analyze_expr(iter)?;
        self.bind_target(target)?;
        self.analyze_body(body)?;
        self.analyze_body(orelse)?;
      }
      Stmt::FunctionDef(function) => self.analyze_function_header(function)?,
      Stmt::Global(names) => {
        for name in names {
          self.bind_global(name)?;
        }
      }
      Stmt::If {
        body,
        clauses,
        test,
      } => self.analyze_if(test, body, clauses)?,
      Stmt::Nonlocal(names) => {
        for name in names {
          self.bind_nonlocal(name)?;
        }
      }
      Stmt::Return(value) => {
        if let Some(value) = value {
          self.analyze_expr(value)?;
        }
      }
      Stmt::While { body, orelse, test } => {
        self.analyze_expr(test)?;
        self.analyze_body(body)?;
        self.analyze_body(orelse)?;
      }
      Stmt::Break | Stmt::Continue | Stmt::Pass => {}
    }

    Ok(())
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

  fn bind_parameters(&mut self, parameters: &[FunctionParameter]) -> Result {
    for parameter in parameters {
      self.bind_parameter(&parameter.name)?;
    }

    Ok(())
  }

  fn bind_target(&mut self, target: &Expr) -> Result {
    match target {
      Expr::List(elements) | Expr::Tuple(elements) => {
        for element in elements {
          self.bind_target(element)?;
        }

        Ok(())
      }
      Expr::Name(name) => self.bind_local(name),
      Expr::Subscript { slice, value } => {
        self.analyze_expr(value)?;
        self.analyze_expr(slice)
      }
      _ => Err(Error::Compile {
        message: "invalid assignment target".into(),
      }),
    }
  }

  fn bind_use(&mut self, name: &str) {
    self.uses.insert(name.to_owned());
  }

  pub(crate) fn function(function: &FunctionDef) -> Result<Self> {
    let mut table = Self {
      kind: ScopeKind::Function,
      ..Self::default()
    };

    table.bind_parameters(&function.parameters)?;

    table.analyze_body(&function.body)?;

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

  fn module(source: &str) -> Module {
    let module = parse(source, Mode::Module.into())
      .unwrap()
      .try_into_module()
      .unwrap()
      .into_syntax();

    LowerAst::new(source).module(&module).unwrap()
  }

  fn function(source: &str) -> FunctionDef {
    match module(source).body.into_iter().next().unwrap() {
      Stmt::FunctionDef(function) => function,
      _ => panic!("expected function definition"),
    }
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

    let error = SymbolTable::function(&function).unwrap_err().to_string();

    assert_eq!(
      error,
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

    assert_eq!(
      SymbolTable::function(&function).unwrap(),
      SymbolTable {
        bindings: HashSet::from(["bar".to_owned()]),
        globals: HashSet::new(),
        kind: ScopeKind::Function,
        local_indices: HashMap::from([("bar".to_owned(), 0)]),
        locals: vec!["bar".to_owned()],
        nonlocals: HashSet::new(),
        uses: HashSet::new(),
      }
    );
  }

  #[test]
  fn module_assignment_resolves_as_name() {
    let module = module("foo = 1");

    assert_eq!(
      SymbolTable::module(&module.body).unwrap(),
      SymbolTable {
        bindings: HashSet::from(["foo".to_owned()]),
        globals: HashSet::new(),
        kind: ScopeKind::Module,
        local_indices: HashMap::new(),
        locals: Vec::new(),
        nonlocals: HashSet::new(),
        uses: HashSet::new(),
      }
    );
  }
}
