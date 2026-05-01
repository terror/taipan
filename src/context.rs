use super::*;

#[derive(Default)]
pub(crate) struct Context<'a> {
  ast: Option<Module>,
  code: Option<Code>,
  module: Option<&'a ModModule>,
  source: Option<&'a str>,
  symbols: Option<SymbolTable>,
}

impl<'a> Context<'a> {
  pub(crate) fn ast(&self) -> Result<&Module> {
    self.ast.as_ref().ok_or_else(|| Error::Compile {
      message: "compiler pipeline did not lower module".into(),
    })
  }

  pub(crate) fn build(self) -> Result<Self> {
    if self.module.is_none() {
      return Err(Error::Compile {
        message: "compiler context missing module".into(),
      });
    }

    if self.source.is_none() {
      return Err(Error::Compile {
        message: "compiler context missing source".into(),
      });
    }

    Ok(self)
  }

  pub(crate) fn code(self) -> Result<Code> {
    self.code.ok_or_else(|| Error::Compile {
      message: "compiler pipeline did not emit bytecode".into(),
    })
  }

  pub(crate) fn module(self, module: &'a ModModule) -> Self {
    Self {
      module: Some(module),
      ..self
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

  pub(crate) fn source(self, source: &'a str) -> Self {
    Self {
      source: Some(source),
      ..self
    }
  }

  pub(crate) fn source_text(&self) -> &'a str {
    self
      .source
      .expect("compiler context should have source after build")
  }

  pub(crate) fn syntax(&self) -> &'a ModModule {
    self
      .module
      .expect("compiler context should have module after build")
  }

  pub(crate) fn take_symbols(&mut self) -> Result<SymbolTable> {
    self.symbols.take().ok_or_else(|| Error::Compile {
      message: "compiler pipeline did not collect symbols".into(),
    })
  }
}
