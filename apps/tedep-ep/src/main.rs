use std::{future::Future, sync::Arc, time::Duration};

use axum::{http::StatusCode, response::IntoResponse, Json, Router};
use clap::{Args, Parser, Subcommand};
use futures::StreamExt;
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::ListParams,
    runtime::{controller::Action, finalizer, Controller},
    CustomResource, CustomResourceExt, ResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use tracing::{error, info, log::warn};
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct MainArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Run(RunArgs),
    GenerateCrds(GenerateCrdsArgs),
}

#[derive(Args, Debug)]
struct RunArgs {
    #[arg(long, short)]
    namespace_str: Option<String>,
    #[arg(long)]
    reconcile_interval: u64,
    #[arg(long)]
    retry_interval: u64,
}

#[derive(Args, Debug)]
struct GenerateCrdsArgs {}

#[tokio::main]
async fn main() {
    let args = MainArgs::parse();
    match args.command {
        Command::Run(args) => {
            let logger = tracing_subscriber::fmt::layer();
            let env_filter = EnvFilter::try_from_default_env().unwrap_or(EnvFilter::new("info"));

            let collector = Registry::default().with(logger).with(env_filter);

            tracing::subscriber::set_global_default(collector).unwrap();

            let namespace_str = args.namespace_str.unwrap();
            let client = kube::Client::try_default().await.unwrap();
            let (controller, state) = init_tfr_controller(
                client,
                namespace_str,
                Duration::from_secs(args.reconcile_interval),
                Duration::from_secs(args.retry_interval),
            )
            .await
            .unwrap();

            let http_app = Router::new()
                .route(
                    "/",
                    axum::routing::get({
                        let state = state.clone();
                        move || index(state.clone())
                    }),
                )
                .route(
                    "/health",
                    axum::routing::get({
                        let state = state.clone();
                        move || health(state.clone())
                    }),
                )
                .route(
                    "/metrics",
                    axum::routing::get({
                        let state = state.clone();
                        move || metrics(state.clone())
                    }),
                )
                .layer(TraceLayer::new_for_http());

            let http_server = axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
                .serve(http_app.into_make_service());

            tokio::select! {
                _ = http_server => info!("http server exited"),
                _ = controller => warn!("controller exited"),
            };
        }
        Command::GenerateCrds(_args) => {
            print!(
                "{}",
                serde_yaml::to_string(&TerraformResource::crd()).unwrap()
            )
        }
    }
}

static TERRAFORM_RESOURCE_FINALIZER: &str = "finalizer.tfr.tedep.quantum-box.com";

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[kube(
    kind = "TerraformResource",
    group = "tedep.quantum-box.com",
    version = "v1",
    namespaced
)]
#[kube(status = "TerraformResourceStatus", shortname = "tfr")]
pub struct TerraformResourceSpec {}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct TerraformResourceStatus {}

#[derive(Default, Clone)]
struct OperatorState;

impl OperatorState {
    fn create_context(
        &self,
        client: kube::Client,
        namespace_str: &str,
        reconcile_interval: Duration,
        retry_interval: Duration,
    ) -> TerraformResourceContext {
        TerraformResourceContext {
            client,
            namespace_str: namespace_str.to_owned(),
            reconcile_interval,
            retry_interval,
        }
    }
}

#[derive(Clone)]
struct TerraformResourceContext {
    client: kube::Client,
    namespace_str: String,
    reconcile_interval: Duration,
    retry_interval: Duration,
}

impl TerraformResourceContext {
    fn client(&self) -> kube::Client {
        self.client.clone()
    }

    fn namespace_str(&self) -> &str {
        &self.namespace_str
    }

    fn reconcile_interval(&self) -> Duration {
        self.reconcile_interval
    }

    fn retry_interval(&self) -> Duration {
        self.retry_interval
    }
}

#[derive(thiserror::Error, Debug)]
enum TerraformResourceError {
    #[error("Kube Error: {0}")]
    KubeError(#[from] kube::Error),
    #[error("Finalizer Error: {0}")]
    FinalizerError(#[from] Arc<kube::runtime::finalizer::Error<Self>>),
}

impl TerraformResource {
    async fn reconcile(
        self: Arc<Self>,
        context: Arc<TerraformResourceContext>,
    ) -> Result<Action, TerraformResourceError> {
        let terraform_resources =
            kube::Api::<TerraformResource>::namespaced(context.client(), context.namespace_str());
        info!("Reconciling TerraformResource \"{}\"", self.name_any());
        finalizer(
            &terraform_resources,
            TERRAFORM_RESOURCE_FINALIZER,
            self,
            |event| async {
                match event {
                    finalizer::Event::Apply(tfr) => tfr.apply((*context).clone()).await,
                    finalizer::Event::Cleanup(tfr) => tfr.cleanup((*context).clone()).await,
                }
            },
        )
        .await
        .map_err(Arc::new)
        .map_err(Into::into)
    }

    async fn apply(
        &self,
        context: TerraformResourceContext,
    ) -> Result<Action, TerraformResourceError> {
        Ok(Action::requeue(context.reconcile_interval()))
    }

    async fn cleanup(
        &self,
        _context: TerraformResourceContext,
    ) -> Result<Action, TerraformResourceError> {
        Ok(Action::await_change())
    }

    fn error_policy(
        self: Arc<Self>,
        error: &TerraformResourceError,
        context: Arc<TerraformResourceContext>,
    ) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(context.retry_interval())
    }
}

async fn init_tfr_controller(
    client: kube::Client,
    namespace_str: String,
    reconcile_interval: Duration,
    retry_interval: Duration,
) -> Result<(impl Future, OperatorState), TerraformResourceError> {
    let state = OperatorState::default();
    {
        let namespace = kube::Api::<Namespace>::all(client.clone());
        let _ = namespace.get(&namespace_str).await?;
    }
    let terraform_resources =
        kube::Api::<TerraformResource>::namespaced(client.clone(), &namespace_str);
    if let Err(e) = terraform_resources
        .list(&ListParams::default().limit(1))
        .await
    {
        error!("CRD is not queryable. Is the CRD installed?");
        return Err(e.into());
    }
    Ok((
        Controller::new(terraform_resources, ListParams::default())
            .run(
                TerraformResource::reconcile,
                TerraformResource::error_policy,
                Arc::new(state.create_context(
                    client,
                    &namespace_str,
                    reconcile_interval,
                    retry_interval,
                )),
            )
            .for_each(|_| futures::future::ready(())),
        state,
    ))
}

async fn index(_state: OperatorState) -> impl IntoResponse {
    (StatusCode::OK, Json(()))
}

async fn health(_state: OperatorState) -> impl IntoResponse {
    (StatusCode::OK, Json("healthy"))
}

async fn metrics(_state: OperatorState) -> impl IntoResponse {
    (StatusCode::OK, Json(()))
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use http::{Request, Response};
    use hyper::Body;
    use kube::{Client, ResourceExt};

    use crate::{TerraformResourceSpec, TERRAFORM_RESOURCE_FINALIZER};

    use super::{TerraformResource, TerraformResourceContext};

    static NS_STR: &str = "tedep";

    #[tokio::test]
    async fn finalizer_creation() {
        let (mock_service, mut handle) = tower_test::mock::pair::<Request<Body>, Response<Body>>();
        let mock_client = Client::new(mock_service, "default");
        let context = TerraformResourceContext {
            client: mock_client,
            namespace_str: NS_STR.to_owned(),
            reconcile_interval: Duration::from_secs(60),
            retry_interval: Duration::from_secs(60),
        };
        let tfr = TerraformResource::new("test-resource", TerraformResourceSpec::default());
        let mock_server = tokio::spawn({
            let mut tfr = tfr.clone();
            async move {
                let (request, sender) = handle.next_request().await.expect("service not called");
                assert_eq!(request.method(), http::Method::PATCH);
                assert_eq!(
                request.uri(),
                "/apis/tedep.quantum-box.com/v1/namespaces/tedep/terraformresources/test-resource"
            );
                let expected_patch = serde_json::json!([
                    { "op": "test", "path": "/metadata/finalizers", "value": null },
                    { "op": "add", "path": "/metadata/finalizers", "value": vec![TERRAFORM_RESOURCE_FINALIZER]}
                ]);
                let request_body = hyper::body::to_bytes(request.into_body()).await.unwrap();
                let runtime_patch: serde_json::Value =
                    serde_json::from_slice(&request_body).unwrap();
                assert_eq!(runtime_patch, expected_patch);

                tfr.finalizers_mut()
                    .push(TERRAFORM_RESOURCE_FINALIZER.to_string());
                let response = serde_json::to_vec(&tfr).unwrap();
                sender.send_response(Response::builder().body(Body::from(response)).unwrap());
            }
        });
        TerraformResource::reconcile(Arc::new(tfr), Arc::new(context))
            .await
            .expect("reconcile failed");
        tokio::time::timeout(Duration::from_secs(1), mock_server)
            .await
            .unwrap()
            .unwrap();
    }
}
