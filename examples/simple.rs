use futures::prelude::*;
use kube::{
    api::{Informer, Object, RawApi, Void, WatchEvent},
    client::APIClient,
    config, Error,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreviewEnvironment {
    pub image: String,
    pub fqdn: String,
}
type KubePreviewEnvironment = Object<PreviewEnvironment, Void>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let kubeconfig = config::load_kube_config().await?;
    let client = APIClient::new(kubeconfig);
    let resource = RawApi::customResource("previewenvironments")
        .group("platform9.com")
        .within("default");
    let informer = Informer::raw(client, resource).init().await?;
    loop {
        let mut previews_stream = informer.poll().await?.boxed();
        while let Some(event) = previews_stream.next().await {
            handle(event?);
        }
    }
}

fn handle(event: WatchEvent<KubePreviewEnvironment>) {
    match event {
        WatchEvent::Added(pe) => println!("Add PreviewEnvironment name: {}", pe.metadata.name),
        WatchEvent::Deleted(pe) => println!("Deleted PreviewEnvironment name: {}", pe.metadata.name),
        WatchEvent::Modified(pe) => println!("Modified PreviewEnvironment name: {}", pe.metadata.name),
        WatchEvent::Error(err) => println!("{:?}", err),
    }
}
