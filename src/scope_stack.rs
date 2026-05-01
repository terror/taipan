use super::*;

pub(crate) struct ScopeStack {
  scopes: Vec<Scope>,
}

impl ScopeStack {
  fn code_mut(&mut self) -> &mut CodeBuilder {
    &mut self.current_mut().code
  }

  pub(crate) fn current(&self) -> &Scope {
    self.scopes.last().unwrap()
  }

  pub(crate) fn current_mut(&mut self) -> &mut Scope {
    self.scopes.last_mut().unwrap()
  }

  pub(crate) fn ensure_capturable(&mut self, name: &str) -> Result {
    match self.current().symbols.resolve(name) {
      Symbol::Local => Ok(()),
      Symbol::Nonlocal => {
        self.free_index(name)?;
        Ok(())
      }
      Symbol::Global | Symbol::Name => {
        if self.has_enclosing_binding(name) {
          self.code_mut().add_freevar(name)?;
          Ok(())
        } else {
          Err(Error::Compile {
            message: format!("no binding for nonlocal '{name}' found"),
          })
        }
      }
    }
  }

  pub(crate) fn enter_function(&mut self, symbols: SymbolTable) {
    self.scopes.push(Scope {
      code: CodeBuilder::default(),
      control_flows: Vec::new(),
      symbols,
    });
  }

  pub(crate) fn exit_scope(&mut self) -> Result<Code> {
    if self.scopes.len() == 1 {
      return Err(Error::Compile {
        message: "cannot exit root compiler scope".into(),
      });
    }

    self.scopes.pop().unwrap().code.finish()
  }

  fn fast_index(&self, name: &str) -> Result<u16> {
    self
      .current()
      .symbols
      .local_index(name)
      .ok_or_else(|| Error::Compile {
        message: format!("missing local: {name}"),
      })
  }

  pub(crate) fn finish(mut self) -> Result<Code> {
    if self.scopes.len() != 1 {
      return Err(Error::Compile {
        message: "unclosed compiler scope".into(),
      });
    }

    self.scopes.pop().unwrap().code.finish()
  }

  pub(crate) fn free_index(&mut self, name: &str) -> Result<u16> {
    if self.has_enclosing_binding(name) {
      self.code_mut().add_freevar(name)
    } else {
      Err(Error::Compile {
        message: format!("no binding for nonlocal '{name}' found"),
      })
    }
  }

  fn has_enclosing_binding(&self, name: &str) -> bool {
    self
      .scopes
      .iter()
      .enumerate()
      .rev()
      .skip(1)
      .any(|(index, scope)| {
        index != 0
          && matches!(
            scope.symbols.resolve(name),
            Symbol::Local | Symbol::Nonlocal
          )
      })
  }

  pub(crate) fn is_module(&self) -> bool {
    self.scopes.len() == 1
  }

  pub(crate) fn module(symbols: SymbolTable) -> Self {
    Self {
      scopes: vec![Scope {
        code: CodeBuilder::default(),
        control_flows: Vec::new(),
        symbols,
      }],
    }
  }

  pub(crate) fn resolve_load(&mut self, name: &str) -> Result<Instruction> {
    match self.current().symbols.resolve(name) {
      Symbol::Local => Ok(Instruction::LoadFast(self.fast_index(name)?)),
      Symbol::Global => {
        Ok(Instruction::LoadName(self.code_mut().add_name(name)?))
      }
      Symbol::Name => {
        if self.has_enclosing_binding(name) {
          Ok(Instruction::LoadFree(self.code_mut().add_freevar(name)?))
        } else {
          Ok(Instruction::LoadName(self.code_mut().add_name(name)?))
        }
      }
      Symbol::Nonlocal => Ok(Instruction::LoadFree(self.free_index(name)?)),
    }
  }

  pub(crate) fn resolve_store(&mut self, name: &str) -> Result<Instruction> {
    match self.current().symbols.resolve(name) {
      Symbol::Global | Symbol::Name => {
        Ok(Instruction::StoreName(self.code_mut().add_name(name)?))
      }
      Symbol::Local => Ok(Instruction::StoreFast(self.fast_index(name)?)),
      Symbol::Nonlocal => Ok(Instruction::StoreFree(self.free_index(name)?)),
    }
  }
}
