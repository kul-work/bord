rem adding SPIN containerd extra flags
docker cp ./.k3d/config.toml.tmpl k3d-bord-server-0:/var/lib/rancher/k3s/agent/etc/containerd/config.toml.tmpl
docker restart k3d-bord-server-0
