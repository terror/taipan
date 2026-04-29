use super::*;

#[derive(Debug, Parser)]
#[command(
  about,
  author,
  version,
  styles = Styles::styled()
    .header(AnsiColor::Green.on_default() | Effects::BOLD)
    .usage(AnsiColor::Green.on_default() | Effects::BOLD)
    .literal(AnsiColor::Blue.on_default() | Effects::BOLD)
    .placeholder(AnsiColor::Cyan.on_default())
)]
pub(crate) struct Arguments {
  #[arg(help = "File to evaluate")]
  filename: PathBuf,
}

impl Arguments {
  pub(crate) fn run(self) -> taipan::Result {
    let source = fs::read_to_string(self.filename)
      .map_err(|source| Error::Io { source })?;

    let parsed = parse(&source, Mode::Module.into())?
      .try_into_module()
      .expect("Mode::Module should produce ModModule");

    let code = Compiler::compile(parsed.syntax())?;

    Machine::run(code)?;

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use {super::*, clap::Parser};

  #[test]
  fn filename() {
    let arguments = Arguments::parse_from(["taipan", "foo.py"]);

    assert_eq!(arguments.filename, PathBuf::from("foo.py"));
  }

  #[test]
  fn missing_filename() {
    assert!(Arguments::try_parse_from(["taipan"]).is_err());
  }

  #[test]
  fn multiple_filenames() {
    assert!(Arguments::try_parse_from(["taipan", "foo.py", "bar.py"]).is_err());
  }
}
