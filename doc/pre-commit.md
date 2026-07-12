<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# `noyafmt` as a pre-commit hook

`noyafmt --check` exits 1 when any of its file arguments would be
rewritten by formatting, printing one path per line on stdout. That
shape matches what `pre-commit`, `lefthook`, and plain Git
`pre-commit` hooks expect of a formatter gate.

## Plain Git hook (`.git/hooks/pre-commit`)

```sh
#!/usr/bin/env sh
set -eu

# Only inspect staged YAML files. `--diff-filter=ACMR` excludes
# deletions and rename-source paths. `-z` + `xargs -0` is robust
# against paths containing spaces.
files=$(git diff --cached --diff-filter=ACMR --name-only -z -- '*.yaml' '*.yml')
[ -z "$files" ] && exit 0

# Run noyafmt --check on every staged YAML file. Any file that needs
# formatting is printed on stdout; we tee it so the developer sees it
# AND we use the exit status to fail the commit.
unformatted=$(printf '%s' "$files" | xargs -0 noyafmt --check || true)
if [ -n "$unformatted" ]; then
    echo "noyafmt: the following files are not formatted:"
    echo "$unformatted" | sed 's/^/    /'
    echo
    echo "fix them with: noyafmt --write \"\$file\""
    exit 1
fi
```

## `pre-commit` framework (`.pre-commit-config.yaml`)

```yaml
repos:
  - repo: local
    hooks:
      - id: noyafmt
        name: noyafmt
        entry: noyafmt --check
        language: system
        types: [yaml]
```

## CI gate (GitHub Actions)

```yaml
- name: noyafmt --check
  run: |
    cargo install noya-cli --version 0.0.15 --locked --no-default-features --features noyafmt
    git ls-files '*.yaml' '*.yml' | xargs noyafmt --check
```

## Editor integration (VS Code, Helix, Zed)

`noyafmt --stdin` reads from stdin and writes to stdout, so any editor
that supports "external formatter" hooks can use it directly:

| Editor    | Setting                                                    |
| --------- | ---------------------------------------------------------- |
| VS Code   | `"yaml.format.command": "noyafmt --stdin"`                  |
| Helix     | `[language.formatter] command = "noyafmt", args = ["--stdin"]` (in `languages.toml`) |
| Zed       | `formatter: { external: { command: "noyafmt", arguments: ["--stdin"] } }` |
