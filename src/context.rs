use super::*;

pub(crate) struct Context<'a> {
  ast: Option<Module>,
  code: Option<Code>,
  module: &'a ModModule,
  symbols: Option<SymbolTable>,
}

impl<'a> Context<'a> {
  pub(crate) fn ast(&self) -> Result<&Module> {
    self.ast.as_ref().ok_or_else(|| Error::Compile {
      message: "compiler pipeline did not lower module".into(),
    })
  }

  pub(crate) fn code(self) -> Result<Code> {
    self.code.ok_or_else(|| Error::Compile {
      message: "compiler pipeline did not emit bytecode".into(),
    })
  }

  pub(crate) fn module(&self) -> &'a ModModule {
    self.module
  }

  pub(crate) fn new(module: &'a ModModule) -> Self {
    Self {
      ast: None,
      code: None,
      module,
      symbols: None,
    }
  }

  pub(crate) fn set_ast(&mut self, ast: Module) {
    self.ast = Some(ast);
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
