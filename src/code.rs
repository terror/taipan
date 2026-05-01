use super::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[typeshare]
pub struct Code {
  #[typeshare(skip)]
  pub constants: Vec<Object>,
  pub freevars: Vec<String>,
  pub instructions: Vec<Instruction>,
  pub locals: Vec<String>,
  pub names: Vec<String>,
}
