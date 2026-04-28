use super::*;

#[derive(Clone, Debug, Default)]
pub struct Code {
  pub constants: Vec<Object>,
  pub locals: Vec<String>,
  pub names: Vec<String>,
  pub ops: Vec<Op>,
}
