use chrono::Utc;
use kube::Config;

pub trait KubeConfigExt {
    fn get_cluster_name(&self) -> String;
}

impl KubeConfigExt for Config {
    fn get_cluster_name(&self) -> String {
        if let Some(q) = self.cluster_url.authority() {
            q.host().replace('.', "_")
        } else {
            Utc::now().to_string()
        }
    }
}
