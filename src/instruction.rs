use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instruction {
  BinaryAdd,
  BinaryDiv,
  BinaryFloorDiv,
  BinaryMod,
  BinaryMul,
  BinaryPow,
  BinarySub,
  BuildString(u16),
  CallFunction(u8),
  CompareEq,
  CompareGe,
  CompareGt,
  CompareLe,
  CompareLt,
  CompareNe,
  Dup,
  Jump(u16),
  LoadConst(u16),
  LoadFast(u16),
  LoadFree(u16),
  LoadName(u16),
  MakeFunction(u16),
  Pop,
  PopJumpIfFalse(u16),
  PopJumpIfTrue(u16),
  Return,
  StoreFast(u16),
  StoreFree(u16),
  StoreName(u16),
  UnaryNeg,
  UnaryNot,
  UnaryPos,
}

impl Serialize for Instruction {
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let mut state = serializer.serialize_struct("Instruction", 2)?;
    state.serialize_field("opcode", self.opcode())?;
    state.serialize_field("argument", &self.argument())?;
    state.end()
  }
}

impl Instruction {
  fn argument(self) -> Option<u16> {
    match self {
      Self::BuildString(argument)
      | Self::Jump(argument)
      | Self::LoadConst(argument)
      | Self::LoadFast(argument)
      | Self::LoadFree(argument)
      | Self::LoadName(argument)
      | Self::MakeFunction(argument)
      | Self::PopJumpIfFalse(argument)
      | Self::PopJumpIfTrue(argument)
      | Self::StoreFast(argument)
      | Self::StoreFree(argument)
      | Self::StoreName(argument) => Some(argument),
      Self::CallFunction(argument) => Some(u16::from(argument)),
      Self::BinaryAdd
      | Self::BinaryDiv
      | Self::BinaryFloorDiv
      | Self::BinaryMod
      | Self::BinaryMul
      | Self::BinaryPow
      | Self::BinarySub
      | Self::CompareEq
      | Self::CompareGe
      | Self::CompareGt
      | Self::CompareLe
      | Self::CompareLt
      | Self::CompareNe
      | Self::Dup
      | Self::Pop
      | Self::Return
      | Self::UnaryNeg
      | Self::UnaryNot
      | Self::UnaryPos => None,
    }
  }

  fn opcode(self) -> &'static str {
    match self {
      Self::BinaryAdd => "binaryAdd",
      Self::BinaryDiv => "binaryDiv",
      Self::BinaryFloorDiv => "binaryFloorDiv",
      Self::BinaryMod => "binaryMod",
      Self::BinaryMul => "binaryMul",
      Self::BinaryPow => "binaryPow",
      Self::BinarySub => "binarySub",
      Self::BuildString(_) => "buildString",
      Self::CallFunction(_) => "callFunction",
      Self::CompareEq => "compareEq",
      Self::CompareGe => "compareGe",
      Self::CompareGt => "compareGt",
      Self::CompareLe => "compareLe",
      Self::CompareLt => "compareLt",
      Self::CompareNe => "compareNe",
      Self::Dup => "dup",
      Self::Jump(_) => "jump",
      Self::LoadConst(_) => "loadConst",
      Self::LoadFast(_) => "loadFast",
      Self::LoadFree(_) => "loadFree",
      Self::LoadName(_) => "loadName",
      Self::MakeFunction(_) => "makeFunction",
      Self::Pop => "pop",
      Self::PopJumpIfFalse(_) => "popJumpIfFalse",
      Self::PopJumpIfTrue(_) => "popJumpIfTrue",
      Self::Return => "return",
      Self::StoreFast(_) => "storeFast",
      Self::StoreFree(_) => "storeFree",
      Self::StoreName(_) => "storeName",
      Self::UnaryNeg => "unaryNeg",
      Self::UnaryNot => "unaryNot",
      Self::UnaryPos => "unaryPos",
    }
  }
}
