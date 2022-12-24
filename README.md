# For Developers

## Running locally

### required tools

- [tilt](https://github.com/tilt-dev/tilt)
- [just](https://github.com/casey/just)

### 1. Create Kubernetes cluster

Set up your cluster locally any way you like.

If you have no preference, we recommend using [ctlptl](https://github.com/tilt-dev/ctlptl) to set up a cluster with a built-in registry.

example:

```
ctlptl create cluster kind --registry=my-registry
```

### 2. Tilt up

```
tilt up
```
