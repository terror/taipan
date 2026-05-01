use super::*;

#[derive(TypedBuilder)]
#[builder(
  builder_method(vis = "pub(crate)"),
  builder_type(vis = "pub(crate)"),
  build_method(vis = "pub(crate)")
)]
pub(crate) struct Context<'a> {
  #[builder(default, setter(skip))]
  ast: Option<Module>,
  #[builder(default, setter(skip))]
  code: Option<Code>,
  module: &'a ModModule,
  source: &'a str,
  #[builder(default, setter(skip))]
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

  pub(crate) fn set_ast(&mut self, ast: Module) {
    self.ast = Some(ast);
  }

  pub(crate) fn set_code(&mut self, code: Code) {
    self.code = Some(code);
  }

  pub(crate) fn set_symbols(&mut self, symbols: SymbolTable) {
    self.symbols = Some(symbols);
  }

  pub(crate) fn source_text(&self) -> &'a str {
    self.source
  }

  pub(crate) fn syntax(&self) -> &'a ModModule {
    self.module
  }

  pub(crate) fn take_symbols(&mut self) -> Result<SymbolTable> {
    self.symbols.take().ok_or_else(|| Error::Compile {
      message: "compiler pipeline did not collect symbols".into(),
    })
  }
}
