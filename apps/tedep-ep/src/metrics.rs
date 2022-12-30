use crate::workspace::metrics::WorkspaceMetrics;

#[derive(Clone)]
pub struct GlobalMetrics {
  _tfws_metrics: WorkspaceMetrics,
}

impl GlobalMetrics {
  pub fn new(tfws_metrics: WorkspaceMetrics) -> Self {
    Self {
      _tfws_metrics: tfws_metrics,
    }
  }
}
