#[derive(Clone, Copy)]
pub(crate) struct ControlFlow {
  pub(crate) break_label: usize,
  pub(crate) break_stack_pops: u16,
  pub(crate) continue_label: usize,
}
