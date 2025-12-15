# Capsules

A tiny helper around Podman for spinning up “capsules” — task-centric containers that keep your host OS clean and your various environments nicely boxed in.

---

## Overview

Capsules is a CLI wrapper for [Podman](https://podman.io/) that makes it easy to:

- Create per-task / per-project containers
- Share only the data you actually care about
- Keep configs, home dirs, and bootstrap scripts organized
- Hop into containers as root or as your regular user

Think of it as a closed-by-default, simpler alternative to distrobox.

---

## Features

- Spin up podman containers as capsules
- Initialize capsules using standard shell scripts
- Run capsules using your host user
- Manage capsules, list, delete, etc.

### Configuration

Capsules looks under `~/.config/capsules` for its config and bootstrap bits.

#### `capsules.toml`

Optional, but handy:

```toml
# ~/.config/capsules/capsules.toml

# Where to store per-capsule volumes (home dirs, bootstrap, etc.)
# If relative, it's resolved from $HOME.
volumes_root = "/files/capsules/volumes"

# What the container considers its "home root"
# (your username is appended, e.g. /home/youruser)
capsule_home_dir = "/home"
```

If you skip this file:

- `volumes_root` defaults to: `~/.local/capsules/volumes`
- `capsule_home_dir` defaults to: `/home`

#### Bootstrap scripts

When you run:

```bash
capsules spin <image> <container_id>
```

Capsules expects a directory:

```text
~/.config/capsules/bootstrap/<container_id>/
```

That directory is copied into the container at:

```text
/files/.bootstrap/
```

Then Capsules runs:

```bash
bash /files/.bootstrap/init.sh
```

So you probably want at least:

```text
~/.config/capsules/bootstrap/<container_id>/init.sh
```

to install packages, create users, tweak configs, etc.

---

### Hard-coded paths (a.k.a. “things you might want to change”)

Right now the code assumes:

- Dotfiles config: `/files/projects/dotfiles/config` mounted to
  `$CAPSULE_HOMEDIR/$USER/.config`
- Fonts: `/files/projects/dotfiles/fonts` mounted to
  `$CAPSULE_HOMEDIR/$USER/.fonts`
- PulseAudio socket: `/run/user/1000/pulse` → `/run/user/host/pulse`

If your setup is different, you’ll probably want to tweak `spin_a_new_capsule` in `main.rs`.

---

## Contribution

Issues, ideas, and PRs are all welcome.

- Found a bug? Open an issue.
- Want another subcommand? Open an issue or draft a PR.
- Have a wild idea for capsule presets, templates, or better defaults? Definitely open an issue.

---

## License

This project is licensed under the BSD 3-Clause License.
See the [LICENSE](LICENSE) file for the boring legal details.
