# Rust Kubernetes Starter Kit

This repo serves as an example of how to create a Kubernetes controller in Rust.  You can use it as a starter template when you need to write a controller for your own CRD's.

## Pre-requisites

- A basic knowledge of [Rust](https://doc.rust-lang.org/book/) is assumed.

- `cargo` and `rustc` are already installed on your machine.

- For development, a `kubeconfig.yaml` is configured such that `kubectl` works on your machine.  Deployed versions will use the in-cluster config taken from mounted secrets in a service account.

- You should have basic Kubernetes knowledge, understand how to create custom [CRD's](https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/), and roughly what a Kubernetes controller does.

- Knowledge of async libraries like `futures` and `tokio` will go a long ways but is not required.

- For the specific demo CRD we will be deploying it is assumed you have Ambassador already working
and have TLS wildcard set up for the hosts you want to use.

## Libraries used

- [`serde`](https://serde.rs/) is used for object / JSON (de)serialization.
- [`kube`](https://github.com/clux/kube-rs) is the Kubernetes client library.
- [`futures`](https://docs.rs/futures/0.3.4/futures/) is the async library 

## Controller Overview

![Controller Overview Diagram](https://github.com/kubernetes/sample-controller/blob/master/docs/images/client-go-controller-interaction.jpeg?raw=true)

Source: https://github.com/kubernetes/sample-controller/blob/master/docs/controller-client-go.md

We will mostly be concerned with writing an `Informer` that watches a custom CRD and responds to `create`, `delete`, `modified`, and `error` change events.

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

- `image` will be the container image we want to deply as a K8s deployment.
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

## Describe the resource you want to watch

## Create an Informer

## Respond to Events


# Cookbook recipes

TODO: Add more code samples on how to do various common tasks such as:

* Create a Helm Chart to deploy the service
* Interact with other K8s resource
* Make API calls
* Connect to an external database
* Kick off a K8s job for an external process (build, deploy, etc)
