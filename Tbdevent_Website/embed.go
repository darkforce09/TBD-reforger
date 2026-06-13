package website

import "embed"

// WebDist holds the production React build. Run `make build-web` before `go build`.
//
//go:embed all:web/dist
var WebDist embed.FS
