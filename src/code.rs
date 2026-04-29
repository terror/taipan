use super::*;

#[derive(Clone, Debug, Default)]
pub struct Code {
  pub constants: Vec<Object>,
  pub instructions: Vec<Instruction>,
  pub locals: Vec<String>,
  pub names: Vec<String>,
}
