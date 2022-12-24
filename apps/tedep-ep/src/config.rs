use std::time::Duration;

use crate::RunArgs;

pub struct GlobalConfig {
    namespace_str: String,
    reconcile_interval: Duration,
    retry_interval: Duration,
}

impl GlobalConfig {
    pub fn namespace_str(&self) -> &str {
        &self.namespace_str
    }
    pub fn reconcile_interval(&self) -> Duration {
        self.reconcile_interval.clone()
    }
    pub fn retry_interval(&self) -> Duration {
        self.retry_interval.clone()
    }

    pub(crate) fn from_run_args(args: &RunArgs) -> Self {
        Self {
            namespace_str: args.namespace_str.to_owned(),
            reconcile_interval: Duration::from_secs(args.reconcile_interval),
            retry_interval: Duration::from_secs(args.retry_interval),
        }
    }
}
