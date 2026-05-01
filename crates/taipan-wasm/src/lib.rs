use {
  taipan::{Compiler, Machine},
  wasm_bindgen::prelude::*,
};

#[wasm_bindgen]
#[derive(Debug, PartialEq)]
pub struct Execution {
  output: String,
  result: String,
}

#[wasm_bindgen]
impl Execution {
  #[must_use]
  #[wasm_bindgen(getter)]
  pub fn output(&self) -> String {
    self.output.clone()
  }

  #[must_use]
  #[wasm_bindgen(getter)]
  pub fn result(&self) -> String {
    self.result.clone()
  }
}

/// Compiles Python source and returns structured bytecode.
///
/// # Errors
///
/// Returns an error if parsing, compilation, or serialization fails.
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<JsValue, JsValue> {
  let code = Compiler::compile_source(source)
    .map_err(|error| JsValue::from_str(&error.to_string()))?;

  serde_wasm_bindgen::to_value(&code)
    .map_err(|error| JsValue::from_str(&error.to_string()))
}

/// Compiles and executes Python source with the virtual machine.
///
/// # Errors
///
/// Returns an error if parsing, compilation, or execution fails.
#[wasm_bindgen]
pub fn execute(source: &str) -> Result<Execution, JsValue> {
  let code = Compiler::compile_source(source)
    .map_err(|error| JsValue::from_str(&error.to_string()))?;

  let (result, output) = Machine::with_output(code, Vec::new())
    .map_err(|error| JsValue::from_str(&error.to_string()))?;

  let output = String::from_utf8(output)
    .map_err(|error| JsValue::from_str(&error.to_string()))?;

  Ok(Execution {
    output,
    result: result.to_string(),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn execute_source_captures_output() {
    assert_eq!(
      execute("print('foo')").unwrap(),
      Execution {
        output: "foo\n".into(),
        result: "None".into(),
      }
    );
  }
}
