# KGV

[![Rust](https://github.com/night-crawler/kgv/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/night-crawler/kgv/actions/workflows/rust.yml)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

`KGV` is Kubernetes Global View or an acronym for GVK. It's a Terminal UI for observing and manipulating Kubernetes resources.

## Features

1. [rhai](https://rhai.rs/)-based data extraction for table views. Write a small script to extract the data for each column for a given GVK. Data extraction is parallel.
2. Rhai module support. Use imports and reuse the code.
3. Detail view templates are defined as Handlebars HTML templates, rendered against the resource context, and then converted to ASCII using [cursive-markup](https://docs.rs/cursive-markup/latest/cursive_markup/).
4. Live reloading support for the rhai engine and templates.
5. Live updates for the detail, YAML, and list views.
6. Collecting initial GVK resource names is cached. Startup on overprovisioned or overloaded k8s installations is faster.
7. Multiple window support. Open multiple windows (logs, list views, list details views, or just details views) and switch between them. Switching is instant.
8. Custom user dirs support. Specify your modules and templates dirs with CMD arguments.

### Minor features

- Rhai debugs are transferred to the main debug window
- Handlebars template includes and inheritance support
- YAML partial code extractors for Handlebars (use `to_yaml` helper)

## Hotkeys

- `~`: Show Debug Console
- `Esc`: Close the current window
- `Ctrl+s`: execute `kubectl exec -it`
- `Alt+=`: Show windows view
- `Ctrl+p`: Dump rhai object to temp
- `F5`: Refresh the view (clears deleted items)
- `Ctrl+y`: Show Resource YAML view 
- `Ctrl+/`: Show a list of registered GVKs
- `Ctrl+k`: Delete current selected resource
- `Ctrl+l`: Show logs for the selected resource

## Screenshots

### Table View: Pods
![image](https://user-images.githubusercontent.com/1235203/229350812-c026d3ec-b90b-4b90-85b2-ea8fb91b7f30.png)

### Table Detail View: Pod containers
![image](https://user-images.githubusercontent.com/1235203/229350859-c73aacb7-bd94-4188-bb5e-2728fb02e02a.png)

See Pseudo Resource extractors in `default_config/views/list/pod.yaml`

### Handlebars HTML Detail View
![image](https://user-images.githubusercontent.com/1235203/229351196-a56c36b2-0cb2-4ab0-9f58-04bf49375687.png)

### YAML View
![image](https://user-images.githubusercontent.com/1235203/229350893-72da2ac5-d723-49b8-8034-4c175d62348e.png)

### Window Switcher View
![image](https://user-images.githubusercontent.com/1235203/229350909-4dcfe45d-c822-4afc-bc52-70173958875e.png)

### GVK Switcher View
![image](https://user-images.githubusercontent.com/1235203/229350941-c635eb76-f40f-46b8-a28c-cf8005b8f9f1.png)

### Logs View
![image](https://user-images.githubusercontent.com/1235203/229350970-85d2fae1-3b23-413b-91fd-027fb76222af.png)

### Menu
![image](https://user-images.githubusercontent.com/1235203/229350996-ca319ac7-f06b-4685-92e1-4fb06d1fc961.png)

## Run

```shell
RUST_BACKTRACE=1 cargo run -- --module-dirs ./default_config/modules --extractor-dirs ./default_config/views/list  --detail-template-dirs ./default_config/views/detail
```

## Adding new Resource / GVK support

1. Describe the Resource List view YAML. Top-level sections are:
    - `resource`: there you describe Group, Version, Kind
    - `imports`: automatically prepend for each column evaluator script these lines
    - `pseudo_resources`: an extractor for nested resource list (return a list of `PseudoResource` items; for `Pod` pseudo-resource is a container)
    - `events`: show either a pseudo resource table or an HTML detail template
    - `columns`: a list of column evaluators with column names
    - `details`: for HTML-based views specify a root template and rhai helpers
2. If you need an HTML detail view, describe templates. Includes and template inheritance is supported.
3. When writing column evaluators, use `Ctrl+P` hotkey to extract currently selected resource as a rhai object.

## TODO

- [ ] Resource multiselect (i.e., to delete multiple resources at once)
- [ ] Resource kill options support
- [x] Faster Log view
- [ ] Better shell selector (now it uses `sh` always)
- [ ] Port Forwarding
- [ ] Configurable hotkeys
- [ ] rhai-based context extractors for rendering with support for multiple resources (when you need to solve N+1 problem for resource detail view and show some dependencies)
- [ ] Log mirroring to a file
- [ ] Prepare more detail and list views for more GVKs
- [x] Support for popular CRDs (helm, GitOps, etc)
- [ ] Solve a problem with panics handler breaking the terminal
