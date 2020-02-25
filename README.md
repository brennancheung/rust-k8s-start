# Rust Kubernetes Starter Kit

This repo serves as an example of how to create a Kubernetes controller in Rust.  You can use it as a starter template when you need to write a controller for your own CRD's.

## Pre-requisites

- A basic knowledge of [Rust](https://doc.rust-lang.org/book/) is assumed.

- `cargo` and `rustc` are already installed on your machine.

- For development, a `kubeconfig.yaml` is configured such that `kubectl` works on your machine.  Deployed versions will use the in-cluster config taken from mounted secrets in a service account.

- You should have basic Kubernetes knowledge, understand how to create custom [CRD's](https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/), and roughly what a Kubernetes controller does.

- Knowledge of async libraries like `futures` and `tokio` will go a long ways but is not required.

- For the specific demo CRD we will be deploying, it is assumed you have Ambassador already working
and have TLS wildcard set up for the hosts you want to use.

## Libraries used

- [`serde`](https://serde.rs/) is used for object / JSON (de)serialization.
- [`kube`](https://github.com/clux/kube-rs) is the Kubernetes client library.
- [`futures`](https://docs.rs/futures/0.3.4/futures/) is the async library 
- [`tokio`](https://tokio.rs/) is the async `executor` runtime used by some of the dependencies so we will stick with it.

## Controller Overview

![Controller Overview Diagram](https://github.com/kubernetes/sample-controller/blob/master/docs/images/client-go-controller-interaction.jpeg?raw=true)

Source: https://github.com/kubernetes/sample-controller/blob/master/docs/controller-client-go.md

We will mostly be concerned with writing an `Informer` that watches a custom CRD and responds to `create`, `delete`, `modified`, and `error` change events.

We will be modifying other Kubernetes objects in our demo controller but what happens here is
entirely based on what you want your controller to do.  You may want to make API calls, modify a database, or include other library crates to help you effect your changes.

# Steps to create your own controller

As a simple demo we will be creating a custom CRD to represent `Preview Environments` designed for
use within a CI/CD system.  After you have a successful build and made a container image you will
want to deploy it somewhere and associate ingress with it.

When you have a PR opened it would be nice if the CI/CD system can add a link in the PR so you can
preview what the change looks like.

It would be nice if creating, modifying, and tearing down a preview environment was a simple as
`kubectl apply -f` and `kubectl delete -f`.


## Create your custom CRD

Our CRD will be a new Kubernetes Object with 2 relevant fields.

- `image` will be the container image we want to deploy as a K8s deployment.
- `fqdn` will be where we want the service to reside.

For the sake of simplicty we will be using Ambassador `Mappings` and assume you already have
Ambassador installed on your cluster.

#### Define the CRD object and schema

First we need to decide on a name for our CRD object and define a schema.

For reference: https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/

Let's create the file `preview-environment-crd.yaml`:

```yaml
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: previewenvironments.platform9.com
spec:
  group: platform9.com
  versions:
    - name: v1
      served: true
      storage: true
      schema:
        openAPIV3Schema:
          type: object
          properties:
            spec:
              type: object
              properties:
                image:
                  type: string
                fqdn:
                  type: string
  scope: Namespaced
  names:
    plural: previewenvironments
    singular: previewenvironment
    kind: PreviewEnvironment
    shortNames:
      - pe
      - previewenv
      - preview
```

Feel free to namespace the API group to a domain you own.

We define 2 fields in the schema (`image` and `fqdn`) and make both of them `string`.
These will show up in the `spec` section of our CRD.


## Define a (de)serializer

Next up let's create a `struct` to represent our CRD.  We will use [serde](https://serde.rs/) as the serialization library
to encode native `struct` objects to and from the actual API calls.

We just need to describe the `spec` section and we can use `serde`'s `derive` macros to automatically build the serializer
and deserializer.  We will also add `Debug` (for printing) and `Clone` capability for convenience.

`serde` supports more sophisticated types like compound types, optional fields, and arrays, but we will stick with a
simple example of just 2 strings for our use case.  You can consult the [serde docs](https://serde.rs/) for more information.

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PreviewEnvironment {
    pub image: String,
    pub fqdn: String,
}
```

Kubernetes objects typically include `metadata`, `spec`, and `status` sections.  This
is so common that the Rust `kube` crate provides a generic type helper
`Object<Spec, Status>`.  Replace `Spec` and `Status` with your custom `struct`s.

```rust
type KubePreviewEnvironment = Object<PreviewEnvironment, Void>;
```

If you are not interested in the `status` section you can just use `Void`.

**Note:** `Void` is a custom type provided by the `kube` crate, not the
typical `void` keyword you might be familiar with from other languages.


## Grab the kubeconfig and create an API client

There isn't much to this step.  We only need 2 lines to do this:

```rust
let kubeconfig = config::load_kube_config().await?;
let client = APIClient::new(kubeconfig);
```

If you are working locally, `load_kube_config` will use your current `kubeconfig`
the same as `kubectl` is currently using.  If you are deploying the service in a pod,
it will use the mounted volume containing your in-cluster credentials from your
`service account`.


## Describe the resource you want to watch

Next we need to define the resource we want to watch.  3 pieces of information are needed:

1) The resource name from the CRD (plural form).
2) The API group to use.
3) The namespace to scope it.

The code to create the resource is simple once you have that information:

```rust
let resource = RawApi::customResource("previewenvironments")
    .group("platform9.com")
    .within("default");
```

This does not actually perform any I/O yet, it is merely defining the resource.


## Create an Informer

To actually watch for change events to our CRD we use what is called an `Informer`.
This is a term commonly used when describing Kubernetes controllers.  It basically
watches for changes and *informs* you whenever it does.

It just needs the `resource` and the existing Kubernetes `client` we defined above.

```rust
let informer = Informer::raw(client, resource).init().await?;
```


## Respond to Events

This part could admittedly be a little cleaner and as of the time of this writing
the Rust async ecosystem is in a state of flux so this part may change in the future.

We need to create a watch loop to receive the change events and dispatch them
to our event handler function.

```rust
loop {
    let mut previews_stream = informer.poll().await?.boxed();
    while let Some(event) = previews_stream.next().await {
        handle(event?);
    }
}
```

The `informer` outputs a `stream` we can use.

Streams are not iterable by default but when we `use futures::prelude::*`, we extend
our code and can use the `Iterator` trait.  From there we can just call `next` to
pull out the event.


## Implement an event handler

The event handler gets passed an event of type `WatchEvent<T>`.  Where `T` is the
type for the Kubernetes CRD object.  In our case it is the `KubePreviewEnvironment`
type we previously defined.

The `handle` function will receive a `WatchEvent` enum consisting of `Added`,
`Deleted`, `Modified`, and `Error`.  If you don't handle all of them the compiler will complain and throw an error.

The first 3 get passed the actual object.  The `WatchEvent::Error` does not and
instead contain an error.

Here is a trivial implementation for a handler function:

```rust
fn handle(event: WatchEvent<KubePreviewEnvironment>) {
    match event {
        WatchEvent::Added(pe) => println!("Add PreviewEnvironment name: {}", pe.metadata.name),
        WatchEvent::Deleted(pe) => println!("Deleted PreviewEnvironment name: {}", pe.metadata.name),
        WatchEvent::Modified(pe) => println!("Modified PreviewEnvironment name: {}", pe.metadata.name),
        WatchEvent::Error(err) => println!("{:?}", err),
    }
}
```


# Using the controller

Let's start up and run the controller:

`cargo run`

After it is done compiling it will be waiting and watching for changes.

Let's create a test `PreviewEnvironment` CRD to add.

```yaml
apiVersion: platform9.com/v1
kind: PreviewEnvironment
metadata:
  name: test-preview-environment
spec:
  image: my-container-image:latest
  fqdn: preview-1.fqdn.com

```

When we run

`kubectl apply -f test-preview-environment.yaml`

and

`kubectl delete -f test-preview-environment.yaml`

We should see:

```
Add PreviewEnvironment name: test-preview-environment
Deleted PreviewEnvironment name: test-preview-environment
```

Hooray, it works!

[Here's a link](examples/simple.rs) to view the entire code in context so far.

Just under 40 LOC!  That's amazing!


# Next steps

If you have something specific in mind for your controller feel free to look
through the cookbook recipes below. 

If not, let's move onto [Interact with other K8s resources](docs/interact-k8s.md)
from the recipes and see how we can manipulate existing Kubernetes objects from
our controller.


# Cookbook recipes

TODO: Add more code samples on how to do various common tasks such as:

* Create a Helm Chart to deploy the service
* [Interact with other K8s resource](docs/interact-k8s.md)
* Make API calls
* Connect to an external database
* Kick off a K8s job for an external process (build, deploy, etc)
* Parse command line options for different options
