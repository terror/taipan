use super::*;

pub(crate) struct Scope {
  pub(crate) code: CodeBuilder,
  pub(crate) symbols: SymbolTable,
}

pub(crate) struct ScopeStack {
  scopes: Vec<Scope>,
}

impl ScopeStack {
  pub(crate) fn current(&self) -> &Scope {
    self.scopes.last().unwrap()
  }

  pub(crate) fn current_mut(&mut self) -> &mut Scope {
    self.scopes.last_mut().unwrap()
  }

  pub(crate) fn enter_function(&mut self, symbols: SymbolTable) {
    self.scopes.push(Scope {
      code: CodeBuilder::default(),
      symbols,
    });
  }

  pub(crate) fn exit_scope(&mut self) -> Result<Code> {
    if self.scopes.len() == 1 {
      return Err(Error::Compile {
        message: "cannot exit root compiler scope".into(),
      });
    }

    Ok(self.scopes.pop().unwrap().code.finish())
  }

  pub(crate) fn finish(mut self) -> Result<Code> {
    if self.scopes.len() != 1 {
      return Err(Error::Compile {
        message: "unclosed compiler scope".into(),
      });
    }

    Ok(self.scopes.pop().unwrap().code.finish())
  }

  pub(crate) fn module(symbols: SymbolTable) -> Self {
    Self {
      scopes: vec![Scope {
        code: CodeBuilder::default(),
        symbols,
      }],
    }
  }
}
