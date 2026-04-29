use {
  arguments::Arguments,
  clap::{
    Parser,
    builder::{
      Styles,
      styling::{AnsiColor, Effects},
    },
  },
  ruff_python_parser::{Mode, parse},
  std::process,
  std::{fs, path::PathBuf},
  taipan::{Compiler, Error, Machine},
};

mod arguments;

fn main() {
  if let Err(error) = Arguments::parse().run() {
    eprintln!("error: {error}");
    process::exit(1);
  }
}
