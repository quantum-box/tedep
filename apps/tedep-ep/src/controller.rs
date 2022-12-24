use std::{borrow::Cow, sync::Arc, time::Duration};

use futures::{future::BoxFuture, FutureExt, StreamExt};
use k8s_openapi::{api::core::v1::Namespace, NamespaceResourceScope};
use kube::{
    api::ListParams,
    runtime::{controller::Action, finalizer},
    ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};
use tracing::info;

use crate::config::GlobalConfig;

#[async_trait::async_trait]
pub trait Controller
where
    <Self::Resource as kube::Resource>::DynamicType:
        Default + std::hash::Hash + std::cmp::Eq + Clone + std::fmt::Debug + Unpin,
{
    type Resource: kube::Resource<Scope = NamespaceResourceScope>
        + Clone
        + DeserializeOwned
        + std::fmt::Debug
        + Send
        + Sync
        + 'static
        + Reconcilable<Context = Self::Context, Error = Self::Error>;
    type Error: From<kube::Error> + Send + Sync + std::error::Error + 'static;
    type Metrics: Default + Send + Sync;
    type Context: Context<Config = Self::Config> + Send + Sync + 'static;
    type Config: Send + Sync + Config;

    async fn init(
        client: kube::Client,
        global_config: &GlobalConfig,
        config: &Self::Config,
    ) -> Result<(BoxFuture<'static, ()>, Self::Metrics), Self::Error>
    where
        Self: Sized,
    {
        let metrics = Self::Metrics::default();
        {
            let namespace = kube::Api::<Namespace>::all(client.clone());
            let _ = namespace.get(&global_config.namespace_str()).await?;
        }
        let resources =
            kube::Api::<Self::Resource>::namespaced(client.clone(), &global_config.namespace_str());
        // check if CRD is available
        let _ = resources.list(&ListParams::default().limit(1)).await?;
        Ok((
            kube::runtime::Controller::new(resources, ListParams::default())
                .run(
                    Self::Resource::reconcile,
                    Self::Resource::error_policy,
                    Arc::new(Self::Context::with_configs(&client, global_config, config)),
                )
                .for_each(|_| futures::future::ready(()))
                .boxed(),
            metrics,
        ))
    }
}

#[async_trait::async_trait]
pub trait Reconcilable
where
    Self: kube::Resource<Scope = NamespaceResourceScope>
        + Clone
        + DeserializeOwned
        + std::fmt::Debug
        + Send
        + Sync
        + 'static
        + Serialize,
    <Self as kube::Resource>::DynamicType:
        Default + std::hash::Hash + std::cmp::Eq + Clone + std::fmt::Debug + Unpin,
{
    const FINALIZER_NAME: &'static str;
    type Context: Context;
    type Error: From<Arc<kube::runtime::finalizer::Error<Self::Error>>>
        + std::error::Error
        + Send
        + Sync
        + 'static;
    async fn reconcile(
        self: Arc<Self>,
        context: Arc<Self::Context>,
    ) -> Result<Action, Self::Error> {
        let resources =
            kube::Api::<Self>::namespaced(context.client().clone(), context.namespace_str());
        info!(
            "Reconciling {} \"{}\"",
            context.resource_kind(),
            self.name_any()
        );
        finalizer(&resources, Self::FINALIZER_NAME, self, |event| async {
            match event {
                finalizer::Event::Apply(tfr) => {
                    info!("Apply {} \"{}\"", context.resource_kind(), tfr.name_any());
                    tfr.apply((*context).clone()).await
                }
                finalizer::Event::Cleanup(tfr) => {
                    info!("Cleanup {} \"{}\"", context.resource_kind(), tfr.name_any());
                    tfr.cleanup((*context).clone()).await
                }
            }
        })
        .await
        .map_err(Arc::new)
        .map_err(Into::into)
    }
    fn error_policy(self: Arc<Self>, error: &Self::Error, context: Arc<Self::Context>) -> Action;
    async fn apply(self: Arc<Self>, context: Self::Context) -> Result<Action, Self::Error>;
    async fn cleanup(self: Arc<Self>, context: Self::Context) -> Result<Action, Self::Error>;
}

pub trait Context: Clone + Send + Sync + 'static {
    type Config: Config;
    fn with_configs(
        client: &kube::Client,
        globa_config: &GlobalConfig,
        config: &Self::Config,
    ) -> Self;

    fn client(&self) -> &kube::Client;
    fn namespace_str(&self) -> &str;
    fn resource_kind(&self) -> Cow<str>;
    fn reconcile_interval(&self) -> Duration;
    fn retry_interval(&self) -> Duration;
}

pub trait Config: Clone {
    type Resource: kube::Resource;
    fn dynamic_type(&self) -> &<Self::Resource as kube::Resource>::DynamicType;
}
