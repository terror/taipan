use super::*;

pub(crate) trait OperatorExt {
  fn instruction(&self) -> Result<Instruction>;
  fn name(&self) -> &'static str;
}

impl OperatorExt for Operator {
  fn instruction(&self) -> Result<Instruction> {
    match self {
      Operator::Add => Ok(Instruction::BinaryAdd),
      Operator::Div => Ok(Instruction::BinaryDiv),
      Operator::FloorDiv => Ok(Instruction::BinaryFloorDiv),
      Operator::Mod => Ok(Instruction::BinaryMod),
      Operator::Mult => Ok(Instruction::BinaryMul),
      Operator::Pow => Ok(Instruction::BinaryPow),
      Operator::Sub => Ok(Instruction::BinarySub),
      _ => Err(Error::UnsupportedSyntax {
        message: format!("operator: {}", self.name()),
      }),
    }
  }

  fn name(&self) -> &'static str {
    match self {
      Operator::Add => "Add",
      Operator::BitAnd => "BitAnd",
      Operator::BitOr => "BitOr",
      Operator::BitXor => "BitXor",
      Operator::Div => "Div",
      Operator::FloorDiv => "FloorDiv",
      Operator::LShift => "LShift",
      Operator::MatMult => "MatMult",
      Operator::Mod => "Mod",
      Operator::Mult => "Mult",
      Operator::Pow => "Pow",
      Operator::RShift => "RShift",
      Operator::Sub => "Sub",
    }
  }
}

impl OperatorExt for BinaryOperator {
  fn instruction(&self) -> Result<Instruction> {
    Ok(match self {
      Self::Add => Instruction::BinaryAdd,
      Self::Div => Instruction::BinaryDiv,
      Self::FloorDiv => Instruction::BinaryFloorDiv,
      Self::Mod => Instruction::BinaryMod,
      Self::Mul => Instruction::BinaryMul,
      Self::Pow => Instruction::BinaryPow,
      Self::Sub => Instruction::BinarySub,
    })
  }

  fn name(&self) -> &'static str {
    match self {
      Self::Add => "Add",
      Self::Div => "Div",
      Self::FloorDiv => "FloorDiv",
      Self::Mod => "Mod",
      Self::Mul => "Mul",
      Self::Pow => "Pow",
      Self::Sub => "Sub",
    }
  }
}
