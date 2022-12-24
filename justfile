default:
  @just --list --unsorted --color=always | rg -v "default"

install-crds:
  cargo run -p tedep-ep -- generate-crds | kubectl apply -f -
