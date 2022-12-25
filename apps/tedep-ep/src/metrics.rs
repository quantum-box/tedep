use crate::tfws::metrics::TerraformWorkspaceMetrics;

#[derive(Clone)]
pub struct GlobalMetrics {
  _tfws_metrics: TerraformWorkspaceMetrics,
}

impl GlobalMetrics {
  pub fn new(
    tfws_metrics: TerraformWorkspaceMetrics,
  ) -> Self {
    Self {
      _tfws_metrics: tfws_metrics,
    }
  }
}
