# Workspace example

A root and three members:

```
workspace/
├── liphia.toml          [workspace]: members + shared requirements
├── app/                 runnable member (entry main.lph)
│   ├── liphia.toml      depends on greet and report
│   └── main.lph
└── libs/
    ├── greet/           pure member, no dependencies
    └── report/          member that depends on the num package
```

Run from this folder:

```bash
liphia members        # what the workspace contains
liphia install        # installs num for the whole workspace
liphia run app        # or: cd app && liphia run
```

Expected output:

```
hello from workspace
latency ms: median 13.5, stdev 7.082843120291926
```

After `liphia install`, `liphia.lock` and `liphia_modules/` exist only here
at the root, never inside a member. `liphia install <pkg>` run inside
`libs/greet/` adds the package to `libs/greet/liphia.toml` and installs it
at the root.

`app` imports `report`, and `report` imports `num`, but `app` itself cannot
`import from "num"`: a member sees only what its own `liphia.toml` declares.
