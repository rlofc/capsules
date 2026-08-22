<p align="center">
  <img src="https://raw.githubusercontent.com/rlofc/capsules/master/capsules.png" alt="Capsules logo" width="128"/>
</p>

# Capsules

[![CI](https://github.com/rlofc/capsules/actions/workflows/ci.yml/badge.svg)](https://github.com/rlofc/capsules/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/capsules.svg)](https://crates.io/crates/capsules)

A tiny CLI wrapper around [Podman](https://podman.io/) for spinning up **capsules** - task-centric containers that keep your __Linux__ host OS clean and your various environments nicely boxed in.

Think of it as a closed-by-default, simpler alternative to distrobox.

---

## What can it do?

- Easily spin up Podman containers as isolated 'capsules'
- Use 'blueprints' as quick templates for the set-up
- Run containers using your host UID (via `--userns=keep-id`)
- Execute commands inside your capsule as your host user
- But have an isolated `home` environment from your host's one
- Root console access for maintenance of capsules
- Sensible defaults for GPU, audio and X11 are built-in

---

## Installation

First, make sure you have `tar` and `podman` installed on your host (Capsules uses the `podman` CLI to implement most of its commands, and uses `tar` when building Dockerfile images)

Then run:

```sh
cargo install capsules
```

And then install the default configuration if you need to:

```
mkdir -p ~/.config/capsules && \
curl -sL https://github.com/rlofc/capsules/archive/master.tar.gz | \
tar -xzf - --wildcards --strip-components=2 --skip-old-files -C ~/.config/capsules '*/capsules/*'
```

## Quick Start

### Basic use

```sh
mkdir my_capsule && cd my_capsule
capsules init debian && capsules create my_capsule && capsules run my_capsule bash
```

### Mounting your workspace

Change `/your/projects/dir` to where your workspace is. It will be mounted as a Podman volume.

```sh
mkdir my_capsule && cd my_capsule
capsules init debian
capsules create my_capsule --volume /your/projects/dir:/your/projects/dir
capsules run my_capsule bash
```

### Recreating a capsule

Changed your mind about the run options after `create`? `recreate` commits the container's current state to a new image, removes the old container, and starts a fresh one from that image using a new set of run args - without losing anything you've installed or configured inside it.

```sh
capsules recreate my_capsule --volume /more/of/your/projects:/more/of/your/projects
```

Since the new container is started from a commit of the old one, `init.sh` is **not** run again.

Useful flags:

- `--with-volumes` - carry over any extra bind mounts (beyond the defaults) the existing container already had
- `--debug` - print the `podman run` command that would be used and confirm before running it
- `--no-gpu` / `--no-pulse` - same meaning as on `create`


## User Guide

### Commands

```
$ capsules --help

Isolated workspace containers for operating-system hygiene

Usage: capsules [COMMAND]

Commands:
  init      Init container volume
  create    Spins up a new container
  recreate  Recreates a container from a commit of its current state, using a new set of run args
  run       Executes a command in a running capsule
  list      List all capsules
  console   Start a console root session
  start     Starts a container
  stop      Stops a container
  delete    Deletes a container
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

```

### Configuration 

Capsules looks under `~/.config/capsules` for its config and blueprints.

#### `capsules.toml`

```toml
# ~/.config/capsules/capsules.toml

# Where will capsules locate your host volume folder
# (This is the folder you used the `capsules init ..` command in)
capsule_volume_dir = "/files"

# What the container considers its "home root". This will be
# appended to the capsule_volume_dir.
# (your username is appended, e.g. /home/youruser)
capsule_home_dir = "home"
```


### Blueprints

Capsules uses __blueprints__ to set up an image and initialization code when spinning up podman containers. 

#### Blueprint directory structure

Blueprints live under `~/.config/capsules/<name>/`. Each blueprint is a directory containing a `Dockerfile`, a `capsule.toml`, and an optional `init.sh`.

```text
~/.config/capsules/<name>/
  Dockerfile       # Required - build context for `podman build`
  capsule.toml     # Required - contains `blueprint = "<name>"`
  init.sh          # Optional - post-start initialization script
```

#### A minimal blueprint:

```text
~/.config/capsules/my-blueprint/
  Dockerfile
  capsule.toml
  init.sh
```

**Dockerfile** - the base image:

```dockerfile
FROM debian:latest
RUN echo "my-blueprint" > /etc/hostname
ENV LANG=en_US.UTF-8
ENV CAPSULE_USERNAME=$CAPSULE_USERNAME

RUN apt-get update && apt-get install -y --no-install-recommends \
    sudo curl bash

RUN echo "$CAPSULE_USERNAME ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers
```

**capsule.toml** - maps this blueprint directory to the Docker image tag:

```toml
blueprint = "my-blueprint"
```

**init.sh** - runs inside the container after it starts (as root), but is mostly
useful for setting things up for your user:

```bash
#!/bin/bash

# Set up your home environment, dotfiles, etc - using something like:
# sudo -u $CAPSULE_USERNAME bash -c '\
#   git clone https://github.com/your/dotfiles \
#   && bash ~/dotfiles/setup.sh
# '
```

The `init.sh` script has access to these environment variables set by Capsules:

| Variable | Description |
|---|---|
| `CAPSULE_USERNAME` | Your host username |
| `CAPSULE_HOMEDIR` | The capsule home root dir (default: `/home`) |

#### Advanced blueprints:

You can add more files to your blueprint and have them be available in your capsule's `.capsules` folder. This is useful if you want to include profiles or additional scripts and use them inside then `init.sh` files.


---

## Contribution

Issues, ideas, and PRs are all welcome.

---

## License

This project is [licensed](LICENSE) under the BSD 3-Clause License.
