use futures::prelude::*;
use kube::{
    api::{Api, Informer, Object, RawApi, Void, WatchEvent, DeleteParams, PostParams},
    client::APIClient,
    config, Error,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use k8s_openapi::api::{
    apps::v1::{DeploymentSpec, DeploymentStatus},
    core::v1::{ServiceSpec, ServiceStatus},
};
type Deployment = Object<DeploymentSpec, DeploymentStatus>;
type Service = Object<ServiceSpec, ServiceStatus>;
type JsonValue = serde_json::value::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreviewEnvironment {
    pub image: String,
    pub fqdn: String,
}
type KubePreviewEnvironment = Object<PreviewEnvironment, Void>;

struct ApiResources {
    client: APIClient,
    deployments: Api<Deployment>,
    services: Api<Service>,
    mappings: RawApi,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let namespace = "default";

    // Attempt to load the kubeconfig.  If kubectl is working with
    // a default config this should work fine.  When deployed inside
    // a pod, it will use the in-cluster config from service account.
    let kubeconfig = config::load_kube_config().await?;

    let client = APIClient::new(kubeconfig);

    // Describe the resource you want to watch.  Note the resource is
    // the using the plural form defined in the CRD.
    let resource = RawApi::customResource("previewenvironments")
        .group("platform9.com")
        .within(namespace);

    let informer = Informer::raw(client.clone(), resource).init().await?;
    let deployments = Api::v1Deployment(client.clone()).within(namespace);
    let services = Api::v1Service(client.clone()).within(namespace);

    let mappings = RawApi::customResource("mappings")
        .group("getambassador.io")
        .version("v2")
        .within(namespace);
    let resources = ApiResources { deployments, services, mappings, client };

    println!("Controller initialized and waiting for changes...");

    loop {
        // There's a bit of advanced Rust features going on here due
        // to lots of async streams, futures, and values typed as Option.
        let mut previews_stream = informer.poll().await?.boxed();
        while let Some(event) = previews_stream.next().await {
            handle(&resources, event?).await;
        }
    }
}

fn json_for_deployment(name: &str) -> JsonValue {
    json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {
            "name": name,
            "labels": {
                "preview": "true",
            }
        },
        "spec": {
            "replicas": 1,
            "selector": {
                "matchLabels": {
                    "app": name,
                }
            },
            "template": {
                "metadata": {
                    "labels": {
                        "app": name,
                    }
                },
                "spec": {
                    "containers": [
                        {
                            "name": name,
                            "image": "nginx"
                        }
                    ]
                }
            }
        }
    })
}

fn json_for_service(name: &str) -> JsonValue {
    json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": name,
            "labels": {
                "preview": "true",
            }
        },
        "spec": {
            "selector": {
                "app": name,
            },
            "ports": [
                {
                    "protocol": "TCP",
                    "port": 80,
                }
            ]
        }
    })
}

fn json_for_mapping(name: &str, host: &str, service: &str) -> JsonValue {
    json!({
        "apiVersion": "getambassador.io/v2",
        "kind": "Mapping",
        "metadata": {
            "name": name,
        },
        "spec": {
            "host": host,
            "service": service,
            "prefix": "/",
        }
    })
}

async fn create_deployment(deployments: &Api<Deployment>, deploy_json: &JsonValue) {
    let pp = PostParams::default();
    let data = serde_json::to_vec(&deploy_json).expect("Failed to serialize Deployment json");
    deployments.create(&pp, data).await.expect("Failed to create deployment");
}

async fn create_service(services: &Api<Service>, service_json: &JsonValue) {
    let pp = PostParams::default();
    let data = serde_json::to_vec(&service_json).expect("Failed to serialize Service json");
    services.create(&pp, data).await.expect("Failed to create service");
}

async fn create_mapping(resources: &ApiResources, mapping_json: &JsonValue) {
    let pp = PostParams::default();
    let data = serde_json::to_vec(&mapping_json).expect("Failed to serialize Mapping json");
    println!("before");
    let request = resources.mappings.create(&pp, data).expect("Failed to create mapping");
    resources.client.request::<Service>(request).await.unwrap();
}

async fn handle(resources: &ApiResources, event: WatchEvent<KubePreviewEnvironment>) {
    match event {
        WatchEvent::Added(pe) => {
            println!("Add PreviewEnvironment name: {}", pe.metadata.name);

            let deploy_name = format!("{}-deployment", pe.metadata.name);
            let service_name = format!("{}-service", pe.metadata.name);
            let mapping_name = format!("{}-mapping", pe.metadata.name);
            let host = format!("{}.volgenic.com", pe.metadata.name);

            // Create a deployment
            let test_deploy = json_for_deployment(deploy_name.as_str());
            create_deployment(&resources.deployments, &test_deploy).await;

            // Create a service
            let test_service = json_for_service(service_name.as_str());
            create_service(&resources.services, &test_service).await;

            // Create a service
            let test_mapping = json_for_mapping(mapping_name.as_str(), host.as_str(), service_name.as_str());
            println!("About to create mapping {:?}", test_mapping);
            println!("Mappings resource {:?}", &resources.mappings);
            create_mapping(&resources, &test_mapping).await;
        }
        WatchEvent::Deleted(pe) => {
            println!("Deleted PreviewEnvironment name: {}", pe.metadata.name);
            resources.services.delete(format!("{}-service", pe.metadata.name).as_str(), &DeleteParams::default()).await.unwrap();
            resources.deployments.delete(format!("{}-deployment", pe.metadata.name).as_str(), &DeleteParams::default()).await.unwrap();
            resources.mappings.delete("test-mapping", &DeleteParams::default()).unwrap();
        },

        WatchEvent::Modified(pe) => println!("Modified PreviewEnvironment name: {}", pe.metadata.name),
        WatchEvent::Error(err) => println!("{:?}", err),
    }
}
