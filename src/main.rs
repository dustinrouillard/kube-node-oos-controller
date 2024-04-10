use anyhow::Ok;
use chrono::Utc;
use futures::TryStreamExt;
use log::info;
use serde_json::json;
use std::{env, pin::pin};

use k8s_openapi::{
    api::core::v1::{Node, Taint},
    apimachinery::pkg::apis::meta::v1::Time,
};
use kube::{
    api::{Api, ObjectMeta, Patch, PatchParams},
    client::Client,
    runtime::{watcher, WatchStreamExt},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env::set_var("RUST_LOG", "info");

    env_logger::init();

    info!("Starting out of service node controller");

    let client = Client::try_default().await?;
    let api = Api::<Node>::all(client);
    let stream = watcher(api, watcher::Config::default())
        .default_backoff()
        .applied_objects();

    let mut stream = pin!(stream);
    while let Some(n) = stream.try_next().await? {
        check_node_available(n).await?;
    }

    Ok(())
}

async fn check_node_available(node: Node) -> anyhow::Result<()> {
    let meta: ObjectMeta = node.metadata;
    let name = match meta.name {
        Some(name) => name,
        None => String::from("Unknown"),
    };

    let spec = node.spec.unwrap();
    let taints = spec.taints.unwrap_or_default();
    let unreachable = taints
        .clone()
        .into_iter()
        .filter(|t| t.key == "node.kubernetes.io/unreachable")
        .any(|_| true);

    let already_out_of_service = taints
        .clone()
        .into_iter()
        .filter(|t| t.key == "node.kubernetes.io/out-of-service")
        .any(|_| true);

    let client = Client::try_default().await?;
    let api: Api<Node> = Api::<Node>::all(client);

    if unreachable && !already_out_of_service {
        info!("{} is no longer reachable, adding the oos taint", name);

        let out_of_service_taint = Taint {
            effect: "NoExecute".to_string(),
            key: "node.kubernetes.io/out-of-service".to_string(),
            value: Some("nodeshutdown".to_string()),
            time_added: Some(Time(Utc::now())),
        };

        let mut new_taints = taints.clone();
        new_taints.extend(vec![out_of_service_taint]);

        let patch = json!({
            "spec": {
                "taints": new_taints
            }
        });

        api.patch(&name, &PatchParams::default(), &Patch::Merge(patch))
            .await?;
    } else if !unreachable && already_out_of_service {
        info!("{} is reachable again, removing the oos taint", name);

        let updated_taints: Vec<Taint> = taints
            .into_iter()
            .filter(|t| t.key != "node.kubernetes.io/out-of-service")
            .collect();

        let patch = json!({
            "spec": {
                "taints": updated_taints
            }
        });

        api.patch(&name, &PatchParams::default(), &Patch::Merge(patch))
            .await?;
    }

    Ok(())
}
