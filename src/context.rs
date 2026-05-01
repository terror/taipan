use super::*;

pub(crate) struct Context<'a> {
  code: Option<Code>,
  module: &'a ModModule,
  symbols: Option<SymbolTable>,
}

impl<'a> Context<'a> {
  pub(crate) fn body(&self) -> &[Stmt] {
    &self.module.body
  }

  pub(crate) fn code(self) -> Result<Code> {
    self.code.ok_or_else(|| Error::Compile {
      message: "compiler pipeline did not emit bytecode".into(),
    })
  }

  pub(crate) fn new(module: &'a ModModule) -> Self {
    Self {
      code: None,
      module,
      symbols: None,
    }
  }

  pub(crate) fn set_code(&mut self, code: Code) {
    self.code = Some(code);
  }

  pub(crate) fn set_symbols(&mut self, symbols: SymbolTable) {
    self.symbols = Some(symbols);
  }

  pub(crate) fn take_symbols(&mut self) -> Result<SymbolTable> {
    self.symbols.take().ok_or_else(|| Error::Compile {
      message: "compiler pipeline did not collect symbols".into(),
    })
  }
}
