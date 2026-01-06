group "default" {
  targets = ["mpc-backend-mock"]
}

target "mpc-backend-mock" {
  dockerfile = "dev-support/containers/ubuntu/Containerfile"
  platforms  = ["linux/amd64"]
  target     = "mpc-backend-mock"
  contexts = {
    rust   = "docker-image://docker.io/library/rust:1.85-slim-bookworm"
    ubuntu = "docker-image://docker.io/library/ubuntu:24.04"
  }
  args = {
    RUSTC_WRAPPER         = "/usr/bin/sccache"
    SCCACHE_GHA_ENABLED   = "off"
    ACTIONS_CACHE_URL     = null
    ACTIONS_RUNTIME_TOKEN = null
  }
  labels = {
    "description"                     = "Container image for mpc-backend-mock"
    "image.type"                      = "final"
    "image.authors"                   = "genesis@zeusnetwork.xyz"
    "image.vendor"                    = "zeusnetwork"
    "image.description"               = "mpc-backend-mock"
    "org.opencontainers.image.source" = "https://github.com/ZeusNetworkHQ/mpc-backend-mock"
  }
}
