rem Step 1: Install cert-manager (required for the operator's webhook system)
kubectl apply -f https://github.com/cert-manager/cert-manager/releases/download/v1.14.3/cert-manager.crds.yaml
helm repo add jetstack https://charts.jetstack.io
helm repo update
helm install cert-manager jetstack/cert-manager --namespace cert-manager --create-namespace --version v1.14.3

rem Step 2: Install KWasm operator (provisions WebAssembly runtime on your nodes)
helm repo add kwasm http://kwasm.sh/kwasm-operator/
helm repo update
helm install kwasm-operator kwasm/kwasm-operator --namespace kwasm --create-namespace  --set kwasmOperator.installerImage=ghcr.io/spinframework/containerd-shim-spin/node-installer:v0.19.0
kubectl annotate node --all kwasm.sh/kwasm-node=true

rem Step 3: Install Spin Operator CRDs and the operator itself
kubectl apply -f https://github.com/spinframework/spin-operator/releases/download/v0.6.1/spin-operator.crds.yaml
helm install spin-operator --namespace spin-operator --create-namespace --version 0.6.1 --wait oci://ghcr.io/spinframework/charts/spin-operator

rem Step 4: Create the shim executor
kubectl apply -f https://github.com/spinframework/spin-operator/releases/download/v0.6.1/spin-operator.shim-executor.yaml
kubectl apply -f https://github.com/spinframework/spin-operator/releases/download/v0.6.1/spin-operator.runtime-class.yaml

rem Step 5: adding SPIN containerd extra flags and restart
docker cp ./.k3d/config.toml.tmpl k3d-bord-server-0:/var/lib/rancher/k3s/agent/etc/containerd/config.toml.tmpl
docker exec k3d-bord-server-0 mkdir -p /etc/containerd
docker exec k3d-bord-server-0 cp /var/lib/rancher/k3s/agent/etc/containerd/config.toml /etc/containerd/config.toml
kubectl label node --all kwasm.sh/kwasm-node=true
docker restart k3d-bord-server-0
timeout /t 5
kubectl delete job -n kwasm k3d-bord-server-0-provision-kwasm