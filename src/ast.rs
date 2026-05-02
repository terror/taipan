#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BinaryOperator {
  Add,
  BitAnd,
  BitOr,
  BitXor,
  Div,
  FloorDiv,
  LShift,
  Mod,
  Mul,
  Pow,
  RShift,
  Sub,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BoolOperator {
  And,
  Or,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CompareOperator {
  Eq,
  Ge,
  Gt,
  In,
  Le,
  Lt,
  Ne,
  NotIn,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Expr {
  Binary {
    lhs: Box<Expr>,
    operator: BinaryOperator,
    rhs: Box<Expr>,
  },
  Bool(bool),
  BoolOp {
    operator: BoolOperator,
    values: Vec<Expr>,
  },
  Call {
    arguments: Vec<Expr>,
    function: Box<Expr>,
    keywords: Vec<KeywordArgument>,
  },
  Compare {
    lhs: Box<Expr>,
    operator: CompareOperator,
    rhs: Box<Expr>,
  },
  Float(f64),
  FormattedString(Vec<Expr>),
  If {
    body: Box<Expr>,
    orelse: Box<Expr>,
    test: Box<Expr>,
  },
  Int(i64),
  List(Vec<Expr>),
  Name(String),
  None,
  String(String),
  Subscript {
    slice: Box<Expr>,
    value: Box<Expr>,
  },
  Tuple(Vec<Expr>),
  Unary {
    operand: Box<Expr>,
    operator: UnaryOperator,
  },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FunctionDef {
  pub(crate) body: Vec<Stmt>,
  pub(crate) name: String,
  pub(crate) parameters: Vec<FunctionParameter>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FunctionParameter {
  pub(crate) default: Option<Expr>,
  pub(crate) name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KeywordArgument {
  pub(crate) name: String,
  pub(crate) value: Expr,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Module {
  pub(crate) body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Stmt {
  AnnAssign {
    target: Expr,
    value: Option<Expr>,
  },
  Assign {
    targets: Vec<Expr>,
    value: Expr,
  },
  AugAssign {
    operator: BinaryOperator,
    target: Expr,
    value: Expr,
  },
  Break,
  Continue,
  Expr(Expr),
  For {
    body: Vec<Stmt>,
    iter: Expr,
    orelse: Vec<Stmt>,
    target: Expr,
  },
  FunctionDef(FunctionDef),
  Global(Vec<String>),
  If {
    body: Vec<Stmt>,
    clauses: Vec<(Option<Expr>, Vec<Stmt>)>,
    test: Expr,
  },
  Nonlocal(Vec<String>),
  Pass,
  Return(Option<Expr>),
  While {
    body: Vec<Stmt>,
    orelse: Vec<Stmt>,
    test: Expr,
  },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UnaryOperator {
  Invert,
  Not,
  UAdd,
  USub,
}
