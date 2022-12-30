use crate::controller::Config;

use super::TerraformWorkspace;

#[derive(Clone, Default)]
pub struct WorkspaceConfig {
  dynamic_type:
    <TerraformWorkspace as kube::Resource>::DynamicType,
}

impl Config for WorkspaceConfig {
  type Resource = TerraformWorkspace;

  fn dynamic_type(
    &self,
  ) -> &<Self::Resource as kube::Resource>::DynamicType {
    &self.dynamic_type
  }
}
