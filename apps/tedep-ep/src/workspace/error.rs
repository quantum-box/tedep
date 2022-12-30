use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
pub enum WorkspaceError {
  #[error("kube-rs error: {0}")]
  KubeError(#[from] kube::Error),
  #[error("Finalizer Error: {0}")]
  FinalizerError(
    #[from] Arc<kube::runtime::finalizer::Error<Self>>,
  ),
}
