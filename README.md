# kalamarnica

Simple and opinionated CLI tool which changes github contexts - Accounts and per-account tokens (permissions).

## Installation

Build from source:

> [!NOTE]
> [MSRV](https://github.com/foresterre/cargo-msrv) is 1.88.0

> [!NOTE]
> `kalamarnica` needs `zlib` for dynamic linker.

```bash
git clone https://github.com/Propfend/kalamarnica.git
cd kalamarnica
cargo build --release
```

## Basic usage

```bash
# Create context using current session information
kalamarnica create --name personal --from-current 

# Create context providing specific information
kalamarnica create --name work --hostname github.com --user myuser --transport https

# Switch context
kalamarnica switch personal

# Display detailed information about all contexts
kalamarnica auth-status
```

## Commands reference

### `list`

List all saved contexts with their configuration. The active context is marked with `*`.

### `current`

Show the active context and any repository-bound context.

### `create`

Create a new context.

| Flag | Description |
|---|---|
| `--name` | Name for the new context |
| `--from-current` | Detect hostname and user from the current `gh` session |
| `--hostname` | GitHub hostname (e.g., `github.com`) |
| `--user` | GitHub username |
| `--transport` | Git transport protocol: `ssh` (default) or `https` |
| `--ssh-host` | SSH config host alias |

Either `--from-current` or both `--hostname` and `--user` are required.

### `switch <name>`

Switch to a context. Applies the stored token and verifies authentication.

### `set-token --name <name> <token>`

Store a per-context GitHub token.

### `delete <name>`

Delete a context and its stored token.

### `bind <name>`

Bind the current repository to a context. Creates a `.ghcontext` file in the repository root.

### `unbind`

Remove the repository context binding.

### `apply`

Apply the repository-bound context (switch to the context specified in `.ghcontext`).

### `auth-status`

Show authentication status for all contexts, including host, user, transport, token, and auth verification.
