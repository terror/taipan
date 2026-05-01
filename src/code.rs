use super::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Code {
  pub constants: Vec<Object>,
  pub freevars: Vec<String>,
  pub instructions: Vec<Instruction>,
  pub locals: Vec<String>,
  pub names: Vec<String>,
}
