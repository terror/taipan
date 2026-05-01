#[derive(Clone, Copy)]
pub(crate) struct ControlFlow {
  pub(crate) break_label: usize,
  pub(crate) continue_label: usize,
}
