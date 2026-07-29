use {
  serde::de::DeserializeOwned,
  std::{collections::BTreeMap, env},
  taipan_lib::kernel::{
    ExecutionMessage, ExecutionRequest, ExecutionState, KernelLaunchSpec,
    LocalKernel, LocalKernelManager,
  },
  tokio::{sync::mpsc, time},
};

fn launch_spec(python: String) -> KernelLaunchSpec {
  KernelLaunchSpec::new(
    vec![
      python,
      "-m".into(),
      "ipykernel_launcher".into(),
      "-f".into(),
      "{connection_file}".into(),
    ],
    BTreeMap::new(),
    "python",
  )
}

fn opaque<T: DeserializeOwned>(value: &str) -> T {
  serde_json::from_value(serde_json::Value::String(value.into())).unwrap()
}

async fn execute(
  manager: &LocalKernelManager,
  kernel_id: taipan_lib::kernel::KernelId,
  events: &mut mpsc::UnboundedReceiver<taipan_lib::kernel::ExecutionEvent>,
  code: &str,
  suffix: &str,
) -> Vec<ExecutionMessage> {
  manager
    .execute(
      kernel_id,
      ExecutionRequest {
        cell_id: opaque(&format!("00000000-0000-4000-8000-0000000000{suffix}")),
        code: code.into(),
        document_id: opaque("00000000-0000-4000-8000-000000000001"),
        execution_id: opaque(&format!(
          "00000000-0000-4000-8000-0000000001{suffix}"
        )),
      },
    )
    .await
    .unwrap();

  time::timeout(std::time::Duration::from_secs(10), async {
    let mut messages = Vec::new();
    let mut idle = false;
    let mut reply = false;

    while !idle || !reply {
      let event = events.recv().await.unwrap();
      idle |= matches!(
        event.message,
        ExecutionMessage::Status {
          execution_state: ExecutionState::Idle
        }
      );
      reply |= matches!(event.message, ExecutionMessage::ExecuteReply { .. });
      messages.push(event.message);
    }

    messages
  })
  .await
  .unwrap()
}

#[tokio::test]
async fn ipython_kernel_reaches_readiness() {
  let Ok(python) = env::var("TAIPAN_IPYTHON_PYTHON") else {
    return;
  };

  let kernel = LocalKernel::launch(launch_spec(python)).await.unwrap();

  assert_eq!(kernel.info().implementation, "ipython");
  assert_eq!(kernel.info().language_info["name"], "python");

  kernel.shutdown().await.unwrap();
}

#[tokio::test]
async fn ipython_executes_values_streams_displays_and_errors() {
  let Ok(python) = env::var("TAIPAN_IPYTHON_PYTHON") else {
    return;
  };
  let (events, mut event_receiver) = mpsc::unbounded_channel();
  let mut manager = LocalKernelManager::default();
  let kernel_id = manager.start_with_events(launch_spec(python), events);

  manager.wait_for_start(kernel_id).await.unwrap();

  let success = execute(
    &manager,
    kernel_id,
    &mut event_receiver,
    "from IPython.display import display\nprint('foo')\ndisplay({'foo': 'bar'})\n42",
    "02",
  )
  .await;

  assert!(success.iter().any(|message| matches!(
    message,
    ExecutionMessage::Stream { name, text }
      if name == "stdout" && text == "foo\n"
  )));
  assert!(success.iter().any(|message| matches!(
    message,
    ExecutionMessage::DisplayData { data, .. }
      if data.contains_key("text/plain")
  )));
  assert!(success.iter().any(|message| matches!(
    message,
    ExecutionMessage::ExecuteResult { data, .. }
      if data.get("text/plain").and_then(serde_json::Value::as_str) == Some("42")
  )));

  let error = execute(
    &manager,
    kernel_id,
    &mut event_receiver,
    "raise RuntimeError('foo')",
    "03",
  )
  .await;

  assert!(error.iter().any(|message| matches!(
    message,
    ExecutionMessage::Error { ename, evalue, .. }
      if ename == "RuntimeError" && evalue == "foo"
  )));

  manager.shutdown(kernel_id).await.unwrap();
}
