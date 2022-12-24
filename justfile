default:
  @just --list --unsorted --color=always | rg -v "default"

generate-crds:
  cargo run -p tedep-ep -- generate-crds > target/crds.yaml
