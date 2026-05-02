use super::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[typeshare]
pub struct Code {
  #[serde(skip)]
  #[typeshare(skip)]
  pub constants: Vec<Object>,
  pub freevars: Vec<String>,
  pub instructions: Vec<Instruction>,
  pub keyword_names: Vec<Vec<String>>,
  pub locals: Vec<String>,
  pub names: Vec<String>,
}
