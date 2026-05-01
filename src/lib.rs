use {
  builtins::BUILTINS,
  code_builder::CodeBuilder,
  expr_ext::ExprExt,
  frame::Frame,
  label::Label,
  num_traits::ToPrimitive,
  operator_ext::OperatorExt,
  ruff_python_ast::{
    Alias, BoolOp, CmpOp, ExceptHandler, Expr, ExprBoolOp, ExprCall,
    ExprCompare, ExprNumberLiteral, ModModule, Number, Operator, Parameters,
    Stmt, StmtAnnAssign, StmtAssign, StmtAugAssign, StmtFunctionDef, StmtIf,
    StmtNonlocal, StmtReturn, StmtWhile, UnaryOp,
  },
  ruff_python_parser::ParseError,
  scope::{Scope, ScopeStack},
  snafu::Snafu,
  std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::{self, Display, Formatter},
    io::{self, Stdout, Write},
    rc::Rc,
  },
  stmt_ext::StmtExt,
  symbol::Symbol,
  symbol_table::SymbolTable,
};

mod builtin;
mod builtins;
mod code;
mod code_builder;
mod compiler;
mod error;
mod expr_ext;
mod frame;
mod function;
mod instruction;
mod label;
mod machine;
mod object;
mod operator_ext;
mod scope;
mod stmt_ext;
mod symbol;
mod symbol_table;

pub type Result<T = (), E = Error> = std::result::Result<T, E>;
pub(crate) type Cell = Rc<RefCell<Option<Object>>>;

pub use {
  builtin::Builtin, code::Code, compiler::Compiler, error::Error,
  function::Function, instruction::Instruction, machine::Machine,
  object::Object,
};
