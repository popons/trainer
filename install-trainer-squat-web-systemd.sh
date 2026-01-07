#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  install-trainer-squat-web-systemd.sh [options]

Options:
  --bin PATH           Path to trainer binary (default: auto-detect)
  --user USER          Service user (default: sudo user or current user)
  --group GROUP        Service group (default: user's primary group)
  --workdir DIR        Working directory (optional)
  --addr HOST:PORT     Bind address (default: trainer's default)
  --duration SECS      Set duration
  --count N            Set count
  --sets N             Set sets
  --interval SECS      Set interval
  --swing-start F      Set swing-start
  --swing-stop F       Set swing-stop
  --freq F             Set freq
  --service-name NAME  Systemd service name (default: trainer-squat-web)
  --dry-run            Print unit file and commands without installing
  -h, --help           Show this help

Examples:
  ./install-trainer-squat-web-systemd.sh
  ./install-trainer-squat-web-systemd.sh --addr 127.0.0.1:12002
  ./install-trainer-squat-web-systemd.sh --bin /usr/local/bin/trainer --user ubuntu
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

service_name="trainer-squat-web"
bin_path=""
svc_user=""
svc_group=""
workdir=""
addr=""
duration=""
count=""
sets=""
interval=""
swing_start=""
swing_stop=""
freq=""
dry_run="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin)
      bin_path="$2"
      shift 2
      ;;
    --user)
      svc_user="$2"
      shift 2
      ;;
    --group)
      svc_group="$2"
      shift 2
      ;;
    --workdir)
      workdir="$2"
      shift 2
      ;;
    --addr)
      addr="$2"
      shift 2
      ;;
    --duration)
      duration="$2"
      shift 2
      ;;
    --count)
      count="$2"
      shift 2
      ;;
    --sets|--set)
      sets="$2"
      shift 2
      ;;
    --interval)
      interval="$2"
      shift 2
      ;;
    --swing-start)
      swing_start="$2"
      shift 2
      ;;
    --swing-stop)
      swing_stop="$2"
      shift 2
      ;;
    --freq)
      freq="$2"
      shift 2
      ;;
    --service-name)
      service_name="$2"
      shift 2
      ;;
    --dry-run)
      dry_run="true"
      shift 1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      usage
      exit 1
      ;;
  esac
done

if [[ "$dry_run" != "true" && $EUID -ne 0 ]]; then
  if command -v sudo >/dev/null 2>&1; then
    exec sudo -E "$0" "$@"
  fi
  echo "Please run as root (or install sudo)."
  exit 1
fi

if [[ -z "$bin_path" ]]; then
  bin_path="$(command -v trainer || true)"
fi
if [[ -z "$bin_path" ]]; then
  for p in /usr/local/bin/trainer /usr/bin/trainer /opt/trainer/trainer; do
    if [[ -x "$p" ]]; then
      bin_path="$p"
      break
    fi
  done
fi
if [[ -z "$bin_path" ]]; then
  echo "trainer binary not found. Use --bin /path/to/trainer"
  exit 1
fi
bin_path="$(readlink -f "$bin_path")"

if [[ -z "$svc_user" ]]; then
  if [[ -n "${SUDO_USER:-}" ]]; then
    svc_user="$SUDO_USER"
  else
    svc_user="$(id -un)"
  fi
fi
if [[ -z "$svc_group" ]]; then
  svc_group="$(id -gn "$svc_user")"
fi

if [[ -n "$workdir" && ! -d "$workdir" ]]; then
  echo "workdir not found: $workdir"
  exit 1
fi

args=(squat-web)
[[ -n "$addr" ]] && args+=("--addr" "$addr")
[[ -n "$duration" ]] && args+=("--duration" "$duration")
[[ -n "$count" ]] && args+=("--count" "$count")
[[ -n "$sets" ]] && args+=("--sets" "$sets")
[[ -n "$interval" ]] && args+=("--interval" "$interval")
[[ -n "$swing_start" ]] && args+=("--swing-start" "$swing_start")
[[ -n "$swing_stop" ]] && args+=("--swing-stop" "$swing_stop")
[[ -n "$freq" ]] && args+=("--freq" "$freq")

exec_start="$bin_path"
for a in "${args[@]}"; do
  exec_start+=" $a"
done

unit_path="/etc/systemd/system/${service_name}.service"
unit_content="$(cat <<EOF
[Unit]
Description=trainer squat-web
After=network.target

[Service]
Type=simple
User=${svc_user}
Group=${svc_group}
EOF
)"

if [[ -n "$workdir" ]]; then
  unit_content+=$'\n'"WorkingDirectory=${workdir}"
fi
unit_content+=$'\n'"ExecStart=${exec_start}"
unit_content+=$'\n'"Restart=on-failure"
unit_content+=$'\n'"RestartSec=2"
unit_content+=$'\n'
unit_content+=$'\n'"[Install]"
unit_content+=$'\n'"WantedBy=multi-user.target"

if [[ "$dry_run" == "true" ]]; then
  echo "Dry run: would write unit to $unit_path"
  echo "----- unit file -----"
  echo "$unit_content"
  echo "----- commands -----"
  echo "systemctl daemon-reload"
  echo "systemctl enable --now $service_name"
  echo "systemctl status --no-pager $service_name"
  echo "journalctl -u $service_name -f"
  exit 0
fi

tmp_file="$(mktemp)"
printf "%s\n" "$unit_content" > "$tmp_file"

install -m 0644 "$tmp_file" "$unit_path"
rm -f "$tmp_file"

systemctl daemon-reload
systemctl enable --now "$service_name"

echo "Installed: $unit_path"
echo "Status: systemctl status --no-pager $service_name"
echo "Logs: journalctl -u $service_name -f"
