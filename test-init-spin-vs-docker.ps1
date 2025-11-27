# timestamp start
$start = Get-Date

# apply manifest
kubectl apply -f bord-spin.yaml
kubectl apply -f bord-service.yaml

# wait until the pods are running & ready
kubectl wait pod -l core.spinkube.dev/app-name=bord --for=condition=Ready --timeout=30s

# timestamp final
$end = Get-Date

Write-Host "Pods App with SPIN startup time: $(( $end - $start ).TotalMilliseconds) ms" -ForegroundColor Green

# delete manifest
kubectl delete -f bord-spin.yaml
kubectl delete -f bord-service.yaml

Write-Host "------------"

# timestamp start
$start = Get-Date

# apply manifest
kubectl apply -f bord-docker.yaml

# wait until the pods are running & ready
kubectl wait pod -l app=bord-docker --for=condition=Ready --timeout=30s

# timestamp final
$end = Get-Date

Write-Host "Pods App with DOCKER startup time: $(( $end - $start ).TotalMilliseconds) ms" -ForegroundColor Green

# delete manifest
kubectl delete -f bord-docker.yaml