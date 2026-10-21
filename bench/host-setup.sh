#!/bin/bash
# One-time host setup for the benchmark runner (needs sudo). Safe to run again.
#
# 1. LXCFS: containers see their own CPU and memory limits in /proc/meminfo, /proc/cpuinfo ...
# 2. cpuset delegation: rootless podman may pin containers to cores (--cpuset-cpus).
# 3. memlock: no limit on locked memory for this user (io_uring, Umbra).
#
# 2 and 3 fully apply to new login sessions. For the current session, 2 is switched on in the
# running cgroup tree below, and 3 needs `sudo prlimit --pid $$ --memlock=unlimited:unlimited`
# in the shell that starts the runner.
#
# 4. With --bot only: the benchmark bot (docs/BOT.md), a systemd user service of this user, installed or
#    updated and (re)started. It runs the bot of this checkout: `git pull` first to update its code. Not
#    by default: only the server that runs our results has the bot, and the GitHub App's key it needs.
#
#    bench/host-setup.sh [user] [--bot]
set -eu

bot=false
users=()
for arg in "$@"; do
    case "$arg" in
        --bot) bot=true ;;
        *) users+=("$arg") ;;
    esac
done
# the user who runs the benchmark: the one who called sudo, if the script runs under sudo
user=${users[0]:-${SUDO_USER:-$(id -un)}}
uid=$(id -u "$user")

if $bot; then
    # before anything changes: the bot is this user's, it needs the app's key, and a restart would end a run
    if [ "$(id -un)" != "$user" ] || [ "$(id -u)" = 0 ]; then
        echo "--bot: run this as $user, without sudo: the bot is a systemd user service of $user" >&2
        exit 1
    fi
    root=$(cd "$(dirname "$0")/.." && pwd)
    config=~/.config/dbo-bot/config.toml
    # the configuration (the example's until there is one) with the bot's defaults: the key, and the work
    # directory with the bot's state
    read -r key work < <(python3 -c '
import os, sys, tomllib
sys.path.insert(0, sys.argv[1])
from bot import DEFAULTS
cfg = dict(DEFAULTS, **tomllib.load(open(sys.argv[2], "rb")))
print(os.path.expanduser(cfg["key_file"]), os.path.expanduser(cfg["work"]))' "$root/tools/bot" \
        "$([ -f "$config" ] && echo "$config" || echo "$root/tools/bot/config.example.toml")")
    if [ ! -r "$key" ]; then
        echo "--bot: no GitHub App key at $key (key_file of the configuration): the bot runs only where its key is" >&2
        exit 1
    fi
    export XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR:-/run/user/$uid}
    if systemctl --user is-active --quiet dbo-bot 2>/dev/null && [ -f "$work/state.json" ] &&
        python3 -c 'import json, sys; sys.exit(0 if json.load(open(sys.argv[1]))["current"] else 1)' "$work/state.json"; then
        echo "--bot: the bot is running a benchmark (\"current\" in $work/state.json): update it when the run is done" >&2
        exit 1
    fi
fi

if ! command -v lxcfs >/dev/null 2>&1; then
    sudo apt-get update -y
    sudo apt-get install -y lxcfs
fi
sudo systemctl enable --now lxcfs

sudo mkdir -p /etc/systemd/system/user@.service.d
sudo tee /etc/systemd/system/user@.service.d/dbolympics-delegate.conf >/dev/null <<'EOF'
[Service]
Delegate=cpu cpuset io memory pids
EOF
sudo systemctl daemon-reload
# the running session: enable cpuset down to the user's manager
for d in /sys/fs/cgroup /sys/fs/cgroup/user.slice "/sys/fs/cgroup/user.slice/user-$uid.slice"; do
    echo +cpuset | sudo tee "$d/cgroup.subtree_control" >/dev/null
done

sudo tee /etc/security/limits.d/dbolympics-memlock.conf >/dev/null <<EOF
$user soft memlock unlimited
$user hard memlock unlimited
EOF

echo "controllers delegated to $user: $(cat "/sys/fs/cgroup/user.slice/user-$uid.slice/user@$uid.service/cgroup.controllers")"
echo "lxcfs: $(ls /var/lib/lxcfs/proc | tr '\n' ' ')"

if $bot; then
    # the configuration stays as it is once there is one; the service runs without a login, and rootless
    # podman then needs the user's lingering
    mkdir -p ~/.config/dbo-bot ~/.config/systemd/user "$work"
    chmod 700 ~/.config/dbo-bot
    if [ ! -f "$config" ]; then
        cp "$root/tools/bot/config.example.toml" "$config"
        echo "bot: wrote $config from tools/bot/config.example.toml"
    fi
    sudo loginctl enable-linger "$user"
    sed "s|%h/dbolympics|$root|g" "$root/tools/bot/dbo-bot.service" > ~/.config/systemd/user/dbo-bot.service
    systemctl --user daemon-reload
    systemctl --user enable --quiet dbo-bot
    systemctl --user restart dbo-bot
    sleep 10
    if ! systemctl --user is-active --quiet dbo-bot; then
        journalctl --user -u dbo-bot -n 20 --no-pager >&2
        exit 1
    fi
    echo "bot: running the bot of $root ($(git -C "$root" log -1 --format=%h)); its log: journalctl --user -u dbo-bot -f"
fi
