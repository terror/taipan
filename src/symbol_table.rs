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
  globals: HashSet<String>,
  kind: ScopeKind,
  locals: Vec<String>,
  nonlocals: HashSet<String>,
}

impl SymbolTable {
  fn analyze_body(&mut self, body: &[Stmt]) -> Result {
    for stmt in body {
      self.analyze_stmt(stmt)?;
    }

    Ok(())
  }

  fn analyze_stmt(&mut self, stmt: &Stmt) -> Result {
    match stmt {
      Stmt::Assign(node) => {
        for target in &node.targets {
          self.bind_target(target)?;
        }
      }
      Stmt::AugAssign(node) => self.bind_target(&node.target)?,
      Stmt::FunctionDef(node) => self.bind_local(&node.name.id)?,
      Stmt::Global(node) => {
        for name in &node.names {
          self.bind_global(&name.id);
        }
      }
      Stmt::If(node) => {
        self.analyze_body(&node.body)?;

        for clause in &node.elif_else_clauses {
          self.analyze_body(&clause.body)?;
        }
      }
      Stmt::Nonlocal(node) => {
        for name in &node.names {
          self.bind_nonlocal(&name.id);
        }
      }
      Stmt::While(node) => self.analyze_body(&node.body)?,
      _ => {}
    }

    Ok(())
  }

  fn bind_global(&mut self, name: &str) {
    self.locals.retain(|local| local != name);
    self.globals.insert(name.to_owned());
  }

  fn bind_local(&mut self, name: &str) -> Result {
    if self.globals.contains(name) || self.nonlocals.contains(name) {
      return Ok(());
    }

    if self.kind == ScopeKind::Module {
      self.bind_global(name);
      return Ok(());
    }

    if self.locals.iter().any(|local| local == name) {
      return Ok(());
    }

    if self.locals.len() == usize::from(u16::MAX) + 1 {
      return Err(Error::Compile {
        message: "local table overflow".into(),
      });
    }

    self.locals.push(name.to_owned());

    Ok(())
  }

  fn bind_nonlocal(&mut self, name: &str) {
    self.locals.retain(|local| local != name);
    self.nonlocals.insert(name.to_owned());
  }

  fn bind_target(&mut self, target: &Expr) -> Result {
    if let Expr::Name(name) = target {
      self.bind_local(&name.id)?;
    }

    Ok(())
  }

  pub(crate) fn function(parameters: &[String], body: &[Stmt]) -> Result<Self> {
    let mut table = Self {
      kind: ScopeKind::Function,
      ..Self::default()
    };

    for parameter in parameters {
      table.bind_local(parameter)?;
    }

    table.analyze_body(body)?;

    Ok(table)
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
    } else if self.locals.iter().any(|local| local == name) {
      Symbol::Local
    } else {
      Symbol::Name
    }
  }
}
