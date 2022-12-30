use std::{marker::PhantomData, sync::Arc};

use k8s_openapi::NamespaceResourceScope;

use crate::{
  service::{
    EitherOrBoth, KubeReosurceDynamicType, KubeResource,
    Scope,
  },
  Request, Service,
};

pub struct App<S, R, E, SC>
where
  S: Service<R, E, SC>,
  SC: Scope,
{
  service: S,
  _ph: PhantomData<(R, E, SC)>,
}

impl Default for App<(), (), (), ()> {
  fn default() -> Self {
    Self {
      service: (),
      _ph: PhantomData,
    }
  }
}

type ArcServiceSetApp<S, R, E, SC, S2, R2, E2, SC2> = App<
  (Arc<S2>, Arc<S>),
  (R2, R),
  EitherOrBoth<E2, E>,
  (SC2, SC),
>;
type ArcServiceSetNamespacedApp<
  S,
  R,
  E,
  SC,
  S2,
  R2,
  E2,
  SC2,
> = App<
  (Arc<(Arc<S2>, Arc<String>)>, Arc<S>),
  (R2, R),
  EitherOrBoth<E2, E>,
  (SC2, SC),
>;

impl<S, R, E, SC> App<S, R, E, SC>
where
  E: Send + std::fmt::Debug,
  S: Service<R, E, SC>,
  SC: Scope,
{
  pub fn cluster_service<R2, E2, S2>(
    self,
    service: S2,
  ) -> ArcServiceSetApp<S, R, E, SC, S2, R2, E2, ()>
  where
    E2: Send + std::fmt::Debug,
    S: Send + Sync + 'static,
    S2: Service<R2, E2, ()> + Send + Sync + 'static,
  {
    App {
      service: (Arc::new(service), Arc::new(self.service)),
      _ph: PhantomData,
    }
  }

  pub fn namespaced_service<R2, E2, S2, S2Fut, S2Req>(
    self,
    namespace: &str,
    service: S2,
  ) -> ArcServiceSetNamespacedApp<
    S,
    R,
    E,
    SC,
    S2,
    R2,
    E2,
    String,
  >
  where
    E: Send + std::fmt::Debug,
    S: Send + Sync + 'static,
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
    App {
      service: (
        Arc::new((
          Arc::new(service),
          Arc::new(namespace.to_owned()),
        )),
        Arc::new(self.service),
      ),
      _ph: PhantomData,
    }
  }

  pub async fn run(
    self,
    client: kube::Client,
  ) -> impl std::future::Future<Output = ()> {
    let s = Arc::new(self.service);
    s.run(client).await.unwrap()
  }
}
