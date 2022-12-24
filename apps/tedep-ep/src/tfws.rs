use std::sync::Arc;

use kube::{runtime::controller::Action, CustomResource, ResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::controller::{Context, Controller, Reconcilable};

use self::{
    config::TerraformWorkspaceConfig, context::TerraformWorkspaceContext,
    error::TerraformWorkspaceError, metrics::TerraformWorkspaceMetrics,
};

pub mod config;
pub mod context;
pub mod error;
pub mod metrics;

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "TerraformWorkspace",
    group = "tedep.quantum-box.com",
    version = "v1",
    namespaced
)]
#[kube(status = "TerraformWorkspaceStatus", shortname = "tfws")]
pub struct TerraformWorkspaceSpec {
    provider: TerraformWorkspaceProvider,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct TerraformWorkspaceStatus {}

#[derive(Default, Deserialize, Serialize, Debug, Clone, JsonSchema)]
pub struct TerraformWorkspaceProvider {
    name: TerraformWorkspaceProviderName,
}

#[derive(Default, Deserialize, Serialize, Debug, Clone, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum TerraformWorkspaceProviderName {
    #[default]
    TerraformCloud,
}

#[derive(Default, Deserialize, Serialize, Debug, Clone, JsonSchema)]
pub struct TerraformCloudConfig;

#[async_trait::async_trait]
impl Reconcilable for TerraformWorkspace {
    const FINALIZER_NAME: &'static str = "finalizer.tedep.quantum-box.com";
    type Context = TerraformWorkspaceContext;
    type Error = TerraformWorkspaceError;
    fn error_policy(self: Arc<Self>, error: &Self::Error, context: Arc<Self::Context>) -> Action {
        warn!(
            "reconcile failed {} \"{}\": {:?}",
            context.resource_kind(),
            self.name_any(),
            error
        );
        Action::requeue(context.retry_interval())
    }
    async fn apply(self: Arc<Self>, context: Self::Context) -> Result<Action, Self::Error> {
        Ok(Action::requeue(context.reconcile_interval()))
    }
    async fn cleanup(self: Arc<Self>, _context: Self::Context) -> Result<Action, Self::Error> {
        Ok(Action::await_change())
    }
}

pub struct TerraformWorkspaceController;

impl Controller for TerraformWorkspaceController {
    type Resource = TerraformWorkspace;
    type Error = TerraformWorkspaceError;
    type Metrics = TerraformWorkspaceMetrics;
    type Context = TerraformWorkspaceContext;
    type Config = TerraformWorkspaceConfig;
}
