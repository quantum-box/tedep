config.define_string("features", args=True)
cfg = config.parse()
features = cfg.get("features", "")
print("compiling with features: {}".format(features))

local_resource("generate-crds", "just generate-crds", deps="apps/tedep-ep")
k8s_yaml("target/crds.yaml")
docker_build("quantum-box/tedep", ".", dockerfile="Dockerfile")
k8s_yaml(
    helm("charts/tedep-controller", namespace="tedep", values=["local.values.yaml"])
)
k8s_resource("tedep-controller", port_forwards=8080)
