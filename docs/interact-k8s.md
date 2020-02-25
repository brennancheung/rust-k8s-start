# Interacting with Kubernetes Objects

There are 2 different ways to interact with Kubernetes objects depending on if
you are working with one of the standard Kubernetes object or a CRD.

You can use 
The [k8s-openapi](https://github.com/Arnavion/k8s-openapi) for the common
Kubernetes objects, and 
[kube::api::RawAPI](https://docs.rs/kube/0.25.0/kube/api/struct.RawApi.html)
for everything else.


## Generate JSON Data

The `kube` API's expect the object data to be encoded using `serde`.

The
[serde_json](https://docs.serde.rs/serde_json/) crate provides what you need
to conveniently write JSON.

Here's an example of writing the JSON for a Kubernetes `service` object.

```rust
type JsonValue = serde_json::value::Value;

fn json_for_service(name: &str) -> JsonValue {
    json!({
        "apiVersion": "v1",
        "kind": "Service",
        "metadata": {
            "name": name,
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
```


## Creating an object using the common API

The [k8s-openapi](https://github.com/Arnavion/k8s-openapi) provides a convenient
API for interacting with the common Kubernets objects.  If you have a custom
object defined from a CRD you will need to use the 
[kube::api::RawAPI](https://docs.rs/kube/0.25.0/kube/api/struct.RawApi.html).

First we need to define the service:
```rust
let services = Api::v1Service(client.clone()).within(namespace);
```

And then we can make the API call.

```rust
let pp = PostParams::default();
let data = serde_json::to_vec(&service_json).expect("Failed to serialize Service json");
services.create(&pp, data).await.expect("Failed to create service");
```


## Creating an object using RawAPI

Creating a custom object is not quite as convenient and is slighly
different but it is still relatively easily.

First we need to define the resource:

```rust
let mappings = RawApi::customResource("mappings")
    .group("getambassador.io")
    .version("v2")
    .within(namespace);
```

And then you can perform the create call as follows:

```rust
let pp = PostParams::default();
let data = serde_json::to_vec(&mapping_json).expect("Failed to serialize Mapping json");
let request = resources.mappings.create(&pp, data).expect("Failed to create mapping");
client.request::<Void>(request).await.unwrap();
```

Notice that it is slightly different because the `.create` call only returns a `Request`
it does not actually make the request.  You will need to pass it into `client.request`
to perform the actual API call.