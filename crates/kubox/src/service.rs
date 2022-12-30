use std::{sync::Arc, time::Duration};

use futures::{
  future::{self, BoxFuture},
  FutureExt, StreamExt,
};
use k8s_openapi::api::core::v1::Namespace;
use kube::{
  api::ListParams, core::NamespaceResourceScope,
  runtime::controller::Action,
};
use serde::de::DeserializeOwned;

use crate::Request;

pub trait KubeResource<SC>
where
  Self: kube::Resource<Scope = SC>
    + Clone
    + DeserializeOwned
    + std::fmt::Debug
    + Send
    + Sync
    + 'static,
  <Self as kube::Resource>::DynamicType: Default
    + std::hash::Hash
    + std::cmp::Eq
    + Clone
    + std::fmt::Debug
    + Unpin,
{
}

pub trait KubeReosurceDynamicType
where
  Self: Default
    + std::hash::Hash
    + std::cmp::Eq
    + Clone
    + std::fmt::Debug
    + Unpin,
{
}

impl<KRD> KubeReosurceDynamicType for KRD where
  Self: Default
    + std::hash::Hash
    + std::cmp::Eq
    + Clone
    + std::fmt::Debug
    + Unpin
{
}

impl<SC, KR> KubeResource<SC> for KR
where
  Self: kube::Resource<Scope = SC>
    + Clone
    + DeserializeOwned
    + std::fmt::Debug
    + Send
    + Sync
    + 'static,
  <Self as kube::Resource>::DynamicType:
    KubeReosurceDynamicType,
{
}

#[async_trait::async_trait]
pub trait Service<R, E, S>
where
  S: Scope,
{
  async fn run(
    self: Arc<Self>,
    client: kube::Client,
  ) -> Result<BoxFuture<'static, ()>, E>;
}

pub trait Scope {}

impl Scope for String {}
impl Scope for () {}
impl<S, S2> Scope for (S, S2)
where
  S: Scope,
  S2: Scope,
{
}

#[async_trait::async_trait]
impl<R, E, Req, F, Fut> Service<R, E, String>
  for (Arc<F>, Arc<String>)
where
  R: KubeResource<NamespaceResourceScope>,
  <R as kube::Resource>::DynamicType:
    KubeReosurceDynamicType,
  E: From<kube::Error>
    + std::fmt::Debug
    + Send
    + std::error::Error
    + 'static,
  F: Fn(Arc<R>) -> Fut + Send + Sync,
  F: 'static,
  Fut: std::future::Future<Output = Result<Req, E>>
    + Send
    + 'static,
  Req: Request,
{
  async fn run(
    self: Arc<Self>,
    client: kube::Client,
  ) -> Result<BoxFuture<'static, ()>, E> {
    {
      let namespace =
        kube::Api::<Namespace>::all(client.clone());
      let _ = namespace.get(&self.1).await?;
    }
    let resources =
      kube::Api::<R>::namespaced(client.clone(), &self.1);
    let _ = resources
      .list(&ListParams::default().limit(1))
      .await?;
    Ok(
      kube::runtime::Controller::new(
        resources,
        ListParams::default(),
      )
      .run(
        {
          move |r, _c| {
            let result = self.0(r);
            async {
              let _ = result.await?;
              Result::<_, E>::Ok(Action::requeue(
                Duration::from_secs(60),
              ))
            }
          }
        },
        |_r, _e, _c| {
          Action::requeue(Duration::from_secs(60))
        },
        Arc::new(()),
      )
      .for_each(|_| futures::future::ready(()))
      .boxed(),
    )
  }
}

#[async_trait::async_trait]
impl Service<(), (), ()> for () {
  async fn run(
    self: Arc<Self>,
    _: kube::Client,
  ) -> Result<BoxFuture<'static, ()>, ()> {
    Ok(futures::future::ready(()).boxed())
  }
}

#[async_trait::async_trait]
impl<R, E, S, SC, R2, E2, S2>
  Service<(R2, R), EitherOrBoth<E2, E>, ((), SC)>
  for (Arc<S2>, Arc<S>)
where
  E: Send,
  E2: Send,
  S: Service<R, E, SC> + Send + Sync + 'static,
  S2: Service<R2, E2, ()> + Send + Sync + 'static,
  SC: Scope,
{
  async fn run(
    self: Arc<Self>,
    client: kube::Client,
  ) -> Result<BoxFuture<'static, ()>, EitherOrBoth<E2, E>>
  {
    match future::join(
      self.0.clone().run(client.clone()),
      self.1.clone().run(client),
    )
    .await
    {
      | (Err(e2), Err(e)) => {
        return Err(EitherOrBoth::Both(e2, e))
      },
      | (Err(e2), Ok(_)) => {
        return Err(EitherOrBoth::Left(e2))
      },
      | (Ok(_), Err(e)) => {
        return Err(EitherOrBoth::Right(e))
      },
      | (Ok(fut2), Ok(fut)) => {
        return Ok(
          future::join(fut2, fut).map(|_| ()).boxed(),
        )
      },
    }
  }
}

#[async_trait::async_trait]
impl<R, E, S, SC, R2, E2, S2, S2Fut, S2Req>
  Service<(R2, R), EitherOrBoth<E2, E>, (String, SC)>
  for (Arc<(Arc<S2>, Arc<String>)>, Arc<S>)
where
  E: Send,
  S: Service<R, E, SC> + Send + Sync + 'static,
  SC: Scope,
  R2: KubeResource<NamespaceResourceScope>,
  <R2 as kube::Resource>::DynamicType:
    KubeReosurceDynamicType,
  E2: From<kube::Error>
    + std::fmt::Debug
    + Send
    + std::error::Error
    + 'static,
  S2: Fn(Arc<R2>) -> S2Fut + Send + Sync,
  S2: 'static,
  S2Fut: std::future::Future<Output = Result<S2Req, E2>>
    + Send
    + 'static,
  S2Req: Request,
{
  async fn run(
    self: Arc<Self>,
    client: kube::Client,
  ) -> Result<BoxFuture<'static, ()>, EitherOrBoth<E2, E>>
  {
    match future::join(
      self.0.clone().run(client.clone()),
      self.1.clone().run(client),
    )
    .await
    {
      | (Err(e2), Err(e)) => {
        return Err(EitherOrBoth::Both(e2, e))
      },
      | (Err(e2), Ok(_)) => {
        return Err(EitherOrBoth::Left(e2))
      },
      | (Ok(_), Err(e)) => {
        return Err(EitherOrBoth::Right(e))
      },
      | (Ok(fut2), Ok(fut)) => {
        return Ok(
          future::join(fut2, fut).map(|_| ()).boxed(),
        )
      },
    }
  }
}

#[derive(Debug)]
pub enum EitherOrBoth<A, B> {
  Left(A),
  Right(B),
  Both(A, B),
}
