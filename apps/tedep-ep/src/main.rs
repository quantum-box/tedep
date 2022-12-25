use axum::{http::StatusCode, response::IntoResponse, Json, Router};
use clap::{Args, Parser, Subcommand};
use kube::CustomResourceExt;
use tower_http::trace::TraceLayer;
use tracing::{info, log::warn};
use tracing_subscriber::{prelude::*, EnvFilter, Registry};

use crate::{
    config::GlobalConfig,
    metrics::GlobalMetrics,
    tfws::{config::TerraformWorkspaceConfig, TerraformWorkspace, TerraformWorkspaceController},
};

mod config;
mod controller;
mod metrics;
mod tfws;

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
    namespace_str: String,
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
            let env_filter =
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

            let collector = Registry::default().with(logger).with(env_filter);

            tracing::subscriber::set_global_default(collector).unwrap();

            let client = kube::Client::try_default().await.unwrap();
            let global_config = GlobalConfig::from_run_args(&args);
            let tfws_config = TerraformWorkspaceConfig::default();
            let (tfws_controller, tfws_metrics) =
                <TerraformWorkspaceController as controller::Controller>::init(
                    client,
                    &global_config,
                    &tfws_config,
                )
                .await
                .unwrap();
            let metrics = GlobalMetrics::new(tfws_metrics);

            let http_app = Router::new()
                .route(
                    "/",
                    axum::routing::get({
                        let metrics = metrics.clone();
                        move || index(metrics.clone())
                    }),
                )
                .route(
                    "/health",
                    axum::routing::get({
                        let metrics = metrics.clone();
                        move || get_health(metrics.clone())
                    }),
                )
                .route(
                    "/metrics",
                    axum::routing::get({
                        let metrics = metrics.clone();
                        move || get_metrics(metrics.clone())
                    }),
                )
                .layer(TraceLayer::new_for_http());

            let http_server = axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
                .serve(http_app.into_make_service());

            tokio::select! {
                _ = http_server => info!("http server exited"),
                _ = tfws_controller => warn!("terraform workspace controller exited"),
            };
        }
        Command::GenerateCrds(_args) => {
            print!(
                "{}",
                serde_yaml::to_string(&TerraformWorkspace::crd()).unwrap()
            )
        }
    }
}

async fn index(_metrics: GlobalMetrics) -> impl IntoResponse {
    (StatusCode::OK, Json(()))
}

async fn get_health(_metrics: GlobalMetrics) -> impl IntoResponse {
    (StatusCode::OK, Json("healthy"))
}

async fn get_metrics(_metrics: GlobalMetrics) -> impl IntoResponse {
    (StatusCode::OK, Json(()))
}
