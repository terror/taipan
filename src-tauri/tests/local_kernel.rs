use {
  std::{collections::BTreeMap, env},
  taipan_lib::kernel::{KernelLaunchSpec, LocalKernel},
};

#[tokio::test]
async fn ipython_kernel_reaches_readiness() {
  let Ok(python) = env::var("TAIPAN_IPYTHON_PYTHON") else {
    return;
  };

  let spec = KernelLaunchSpec::new(
    vec![
      python,
      "-m".into(),
      "ipykernel_launcher".into(),
      "-f".into(),
      "{connection_file}".into(),
    ],
    BTreeMap::new(),
    "python",
  );

  let kernel = LocalKernel::launch(spec).await.unwrap();

  assert_eq!(kernel.info().implementation, "ipython");
  assert_eq!(kernel.info().language_info["name"], "python");

  kernel.shutdown().await.unwrap();
}
