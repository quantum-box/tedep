use std::{borrow::Cow, time::Duration};

use crate::controller::{Config, Context};

use super::{
  config::TerraformWorkspaceConfig, TerraformWorkspace,
};

#[derive(Clone)]
pub struct TerraformWorkspaceContext {
  client: kube::Client,
  namespace_str: String,
  reconcile_interval: Duration,
  retry_interval: Duration,
  dynamic_type:
    <TerraformWorkspace as kube::Resource>::DynamicType,
}

impl Context for TerraformWorkspaceContext {
  type Config = TerraformWorkspaceConfig;

  fn with_configs(
    client: &kube::Client,
    global_config: &crate::config::GlobalConfig,
    config: &Self::Config,
  ) -> Self {
    Self {
      client: client.clone(),
      namespace_str: global_config
        .namespace_str()
        .to_owned(),
      reconcile_interval: global_config
        .reconcile_interval(),
      retry_interval: global_config.retry_interval(),
      dynamic_type: config.dynamic_type().to_owned(),
    }
  }

  fn client(&self) -> &kube::Client {
    &self.client
  }
  fn namespace_str(&self) -> &str {
    &self.namespace_str
  }
  fn resource_kind(&self) -> Cow<str> {
    <TerraformWorkspace as kube::Resource>::kind(
      &self.dynamic_type,
    )
  }
  fn reconcile_interval(&self) -> Duration {
    self.reconcile_interval.to_owned()
  }
  fn retry_interval(&self) -> Duration {
    self.retry_interval.to_owned()
  }
}
