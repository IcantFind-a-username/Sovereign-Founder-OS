#!/usr/bin/env bash
# RFC 0005 Program 1A — sole sanitized Cargo qualification entry point for
# sovereign-vault-v2-engine. Rejects ambient dependency-shaping input, builds a
# positive-allowlist child environment, owns a fresh CARGO_TARGET_DIR, and runs
# Cargo only as --frozen --offline after the reviewed lock acquisition step.
#
# Portability: bash 3.2 (stock macOS). No associative arrays, no mapfile.
# A qualification gate that cannot run on a developer machine is not a gate.
set -uo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "qualify-vault-v2: unsupported bash: need 3.2+, got ${BASH_VERSION:-non-bash}" >&2
  exit 3
fi

QUALIFY_STATE=running # running | completed | failed
QUALIFY_STEPS=""
TARGET_DIR=""
MANIFEST=""
REVIEWED_HOME=""
REVIEWED_USERPROFILE=""
REVIEWED_APPDATA=""
REVIEWED_LOCALAPPDATA=""
REVIEWED_SYSTEMROOT=""
REVIEWED_PATH=""

on_exit() {
  local status=$?
  if [ -n "$TARGET_DIR" ] && [ -d "$TARGET_DIR" ]; then
    rm -rf "$TARGET_DIR"
  fi
  if [ "$QUALIFY_STATE" = "running" ]; then
    echo "qualify-vault-v2: aborted before completion (raw exit $status); steps:${QUALIFY_STEPS:- none}" >&2
    if [ "$status" -eq 0 ]; then
      status=1
    fi
    exit "$status"
  fi
  exit "$status"
}
trap on_exit EXIT

die() {
  QUALIFY_STATE=failed
  echo "qualify-vault-v2: $*" >&2
  exit 1
}

note_step() {
  QUALIFY_STEPS="$QUALIFY_STEPS $1"
}

# ---- closed dependency-shaping allowlist (mirrors build_gate.rs) ----------
DEPENDENCY_SHAPING_VARIABLES="
LIBSQLITE3_SYS_USE_PKG_CONFIG
LIBSQLITE3_FLAGS
SQLITE_MAX_VARIABLE_NUMBER
SQLITE_MAX_EXPR_DEPTH
SQLITE_MAX_COLUMN
SQLCIPHER_LIB_DIR
SQLCIPHER_INCLUDE_DIR
SQLCIPHER_STATIC
OPENSSL_NO_VENDOR
OPENSSL_DIR
OPENSSL_LIB_DIR
OPENSSL_INCLUDE_DIR
OPENSSL_CONFIG_DIR
OPENSSL_LIBS
OPENSSL_STATIC
OPENSSL_SRC_PERL
OPENSSL_RUST_USE_NASM
PERL
PERL5OPT
PERL5LIB
VCPKGRS_DYNAMIC
RUSTFLAGS
CARGO_ENCODED_RUSTFLAGS
"

value_is_nonempty() {
  local trimmed
  trimmed=$(echo "$1" | tr -d '[:space:]')
  [ -n "$trimmed" ]
}

name_matches_shaping_tail() {
  local name="$1"
  local candidate="$2"
  local suffix="_${candidate}"
  if [ "$name" = "$candidate" ]; then
    return 0
  fi
  case "$name" in
  *"$suffix") ;;
  *) return 1 ;;
  esac
  local prefix_len=$(( ${#name} - ${#suffix} ))
  [ "$prefix_len" -gt 0 ]
}

is_dependency_shaping_name() {
  local name="$1"
  case "$name" in
  PKG_CONFIG_*) return 0 ;;
  esac
  local candidate
  for candidate in $DEPENDENCY_SHAPING_VARIABLES; do
    if name_matches_shaping_tail "$name" "$candidate"; then
      return 0
    fi
  done
  return 1
}

reject_caller_shaping_env() {
  local rejected=""
  local name value
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    name=${line%%=*}
    value=${line#*=}
    if value_is_nonempty "$value" && is_dependency_shaping_name "$name"; then
      rejected="$rejected $name"
    fi
  done <<EOF
$(env)
EOF
  if [ -n "$rejected" ]; then
    die "refusing caller environment with dependency-shaping override(s):$rejected"
  fi
}

discover_cargo_config_files() {
  local root="$1"
  local cargo_home="$2"
  local dir="$root"
  local found=""
  while [ "$dir" != "/" ]; do
    if [ -f "$dir/.cargo/config.toml" ]; then
      found="$found $dir/.cargo/config.toml"
    fi
    if [ -f "$dir/.cargo/config" ]; then
      found="$found $dir/.cargo/config"
    fi
    dir=$(dirname "$dir")
  done
  if [ -f "$cargo_home/config.toml" ]; then
    found="$found $cargo_home/config.toml"
  fi
  if [ -f "$cargo_home/config" ]; then
    found="$found $cargo_home/config"
  fi
  echo "$found"
}

reject_cargo_configs() {
  local configs
  configs=$(discover_cargo_config_files "$ROOT" "$REVIEWED_CARGO_HOME")
  if [ -n "$(echo "$configs" | tr -d '[:space:]')" ]; then
    die "refusing discovered Cargo configuration file(s):$configs"
  fi
}

reject_caller_config_args() {
  local arg
  for arg in "$@"; do
    case "$arg" in
    --config | --config=*)
      die "caller --config is forbidden for Program 1A qualification"
      ;;
    esac
  done
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "missing required tool on PATH: $1"
}

tool_version() {
  local tool="$1"
  case "$tool" in
  cargo | rustc | rustdoc)
    "$tool" --version 2>/dev/null | head -n 1
    ;;
  cargo-clippy)
    "$tool" --version 2>/dev/null | head -n 1
    ;;
  clippy-driver)
    "$tool" --version 2>/dev/null | head -n 1
    ;;
  cc | gcc)
    "$tool" --version 2>/dev/null | head -n 1
    ;;
  ar | ranlib)
    "$tool" --version 2>/dev/null | head -n 1 || echo "$tool (version unknown)"
    ;;
  perl)
    "$tool" --version 2>/dev/null | head -n 1
    ;;
  make)
    "$tool" --version 2>/dev/null | head -n 1
    ;;
  *)
    echo "unknown"
    ;;
  esac
}

tool_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo "unavailable"
  fi
}

resolve_tools() {
  require_cmd cargo
  require_cmd rustc
  require_cmd rustdoc
  require_cmd cc
  require_cmd ar
  require_cmd ranlib
  require_cmd perl
  require_cmd make

  ABS_CARGO=$(command -v cargo)
  ABS_RUSTC=$(command -v rustc)
  ABS_RUSTDOC=$(command -v rustdoc)
  ABS_CC=$(command -v cc)
  ABS_AR=$(command -v ar)
  ABS_RANLIB=$(command -v ranlib)
  ABS_PERL=$(command -v perl)
  ABS_MAKE=$(command -v make)

  ABS_CARGO_CLIPPY=$(command -v cargo-clippy 2>/dev/null || true)
  if [ -z "$ABS_CARGO_CLIPPY" ]; then
    if [ -x "${ABS_CARGO%/*}/cargo-clippy" ]; then
      ABS_CARGO_CLIPPY="${ABS_CARGO%/*}/cargo-clippy"
    else
      die "cargo-clippy not found beside cargo and not on PATH"
    fi
  fi

  ABS_CLIPPY_DRIVER=$(command -v clippy-driver 2>/dev/null || true)
  if [ -z "$ABS_CLIPPY_DRIVER" ]; then
    if [ -x "${ABS_RUSTC%/*}/clippy-driver" ]; then
      ABS_CLIPPY_DRIVER="${ABS_RUSTC%/*}/clippy-driver"
    else
      die "clippy-driver not found beside rustc and not on PATH"
    fi
  fi

  local sibling="${ABS_CARGO_CLIPPY%/*}/clippy-driver"
  if [ ! -x "$sibling" ]; then
    die "cargo-clippy directory has no clippy-driver sibling"
  fi
  if [ "$sibling" != "$ABS_CLIPPY_DRIVER" ]; then
    die "clippy-driver on PATH ($ABS_CLIPPY_DRIVER) does not match cargo-clippy sibling ($sibling)"
  fi
}

admitted_engine_target() {
  echo "${SFO_VAULT_PLATFORM_ADMITTED_TARGET:-x86_64-unknown-linux-gnu}"
}

verify_host_triple() {
  local host_line host admitted
  host_line=$(rustc -vV 2>/dev/null | grep '^host:' || true)
  host=${host_line#host: }
  host=$(echo "$host" | tr -d '[:space:]')
  admitted=$(admitted_engine_target)
  case "$admitted" in
  x86_64-unknown-linux-gnu | aarch64-apple-darwin | x86_64-pc-windows-msvc) ;;
  *)
    die "unknown admitted engine target: $admitted"
    ;;
  esac
  if [ "$host" != "$admitted" ]; then
    die "host ($host) must match admitted target ($admitted)"
  fi
}

write_manifest_header() {
  local path="$1"
  : >"$path"
  {
    echo "# qualify-vault-v2 evidence manifest"
    echo "# generated: $(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u)"
    echo "# repository: $ROOT"
    echo "# cargo_target_dir: $TARGET_DIR"
    echo "# admitted_target: $(admitted_engine_target)"
    echo
    echo "## reviewed tools"
  } >>"$path"
}

record_tool_line() {
  local label="$1"
  local path="$2"
  printf '%s path=%s version=%s sha256=%s\n' \
    "$label" "$path" "$(tool_version "$(basename "$path")")" "$(tool_sha256 "$path")" >>"$MANIFEST"
}

emit_tool_manifest() {
  write_manifest_header "$MANIFEST"
  record_tool_line "cargo" "$ABS_CARGO"
  record_tool_line "rustc" "$ABS_RUSTC"
  record_tool_line "rustdoc" "$ABS_RUSTDOC"
  record_tool_line "cargo-clippy" "$ABS_CARGO_CLIPPY"
  record_tool_line "clippy-driver" "$ABS_CLIPPY_DRIVER"
  record_tool_line "cc" "$ABS_CC"
  record_tool_line "ar" "$ABS_AR"
  record_tool_line "ranlib" "$ABS_RANLIB"
  record_tool_line "perl" "$ABS_PERL"
  record_tool_line "make" "$ABS_MAKE"
  echo >>"$MANIFEST"
}

ensure_frozen_offline_args() {
  # Mutates CARGO_ARGS array in caller via nameref simulation: print to stdout
  local has_frozen=0 has_offline=0 arg
  for arg in "$@"; do
    case "$arg" in
    --frozen) has_frozen=1 ;;
    --offline) has_offline=1 ;;
    esac
  done
  if [ "$has_frozen" -eq 0 ] || [ "$has_offline" -eq 0 ]; then
    die "cargo qualification commands must include --frozen and --offline"
  fi
}

qualification_child_home() {
  if [ -n "${SFO_VAULT_PLATFORM_NAMESPACE:-}" ]; then
    echo "${SFO_QUALIFY_HOME:-$REVIEWED_HOME}"
  else
    echo "$FRESH_HOME"
  fi
}

run_child() {
  local log_label="$1"
  shift
  local -a cmd=("$@")
  local child_home
  child_home=$(qualification_child_home)
  note_step "$log_label"
  echo "==== [$log_label] ${cmd[*]}" >>"$QUALIFY_LOG"
  set -- env -i \
    HOME="$child_home" \
    TMPDIR="$FRESH_TMP" \
    LANG="${LANG:-C.UTF-8}" \
    LC_ALL="${LC_ALL:-C.UTF-8}" \
    SOURCE_DATE_EPOCH=1700000000 \
    CARGO_HOME="$REVIEWED_CARGO_HOME" \
    RUSTUP_HOME="${RUSTUP_HOME:-}" \
    CARGO="$ABS_CARGO" \
    RUSTC="$ABS_RUSTC" \
    RUSTDOC="$ABS_RUSTDOC" \
    SOVEREIGN_CARGO_CLIPPY="$ABS_CARGO_CLIPPY" \
    SOVEREIGN_CLIPPY_DRIVER="$ABS_CLIPPY_DRIVER" \
    CARGO_NET_OFFLINE=true \
    CARGO_TARGET_DIR="$TARGET_DIR" \
    SFO_VAULT_PLATFORM_NAMESPACE="${SFO_VAULT_PLATFORM_NAMESPACE:-}" \
    SFO_VAULT_PLATFORM_ADMITTED_TARGET="${SFO_VAULT_PLATFORM_ADMITTED_TARGET:-}" \
    DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-}" \
    XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-}" \
    PATH="$CHILD_PATH"
  if [ -n "${SFO_VAULT_PLATFORM_NAMESPACE:-}" ]; then
    [ -n "$REVIEWED_USERPROFILE" ] && set -- "$@" USERPROFILE="$REVIEWED_USERPROFILE"
    [ -n "$REVIEWED_APPDATA" ] && set -- "$@" APPDATA="$REVIEWED_APPDATA"
    [ -n "$REVIEWED_LOCALAPPDATA" ] && set -- "$@" LOCALAPPDATA="$REVIEWED_LOCALAPPDATA"
    [ -n "$REVIEWED_SYSTEMROOT" ] && set -- "$@" SYSTEMROOT="$REVIEWED_SYSTEMROOT"
  fi
  set -- "$@" "${cmd[@]}"
  "$@" >>"$QUALIFY_LOG" 2>&1 || die "$log_label failed (see $QUALIFY_LOG)"
}

append_child_path_dir() {
  local dir="$1"
  [ -n "$dir" ] || return 0
  case ":$CHILD_PATH:" in
  *":$dir:"*) ;;
  *) CHILD_PATH="${CHILD_PATH:+$CHILD_PATH:}$dir" ;;
  esac
}

append_runner_msvc_dirs_to_child_path() {
  [ -n "${SFO_VAULT_PLATFORM_NAMESPACE:-}" ] || return 0
  local uname_s path_sep entry old_ifs
  uname_s=$(uname -s 2>/dev/null || echo unknown)
  case "$uname_s" in
  MINGW* | MSYS* | CYGWIN* | Windows_nt | Windows_NT) ;;
  *) return 0 ;;
  esac
  path_sep=":"
  case "$REVIEWED_PATH" in
  *\;*) path_sep=";" ;;
  esac
  old_ifs="$IFS"
  IFS="$path_sep"
  for entry in $REVIEWED_PATH; do
    case "$entry" in
    *" "*) continue ;;
    "") continue ;;
    esac
    if [ -f "$entry/link.exe" ] || [ -f "$entry/link.EXE" ] ||
      [ -f "$entry/cl.exe" ] || [ -f "$entry/cl.EXE" ] ||
      [ -f "$entry/lib.exe" ] || [ -f "$entry/lib.EXE" ]; then
      append_child_path_dir "$entry"
    fi
  done
  IFS="$old_ifs"
}

build_child_path() {
  CHILD_PATH=""
  local dir
  for dir in \
    "${ABS_CARGO%/*}" \
    "${ABS_RUSTC%/*}" \
    "${ABS_CC%/*}" \
    "${ABS_PERL%/*}" \
    "${ABS_MAKE%/*}"; do
    append_child_path_dir "$dir"
  done
  if [ -d /usr/bin ]; then
    append_child_path_dir /usr/bin
  fi
  append_runner_msvc_dirs_to_child_path
}

run_cargo_child() {
  local label="$1"
  shift
  ensure_frozen_offline_args "$@"
  run_child "$label" "$ABS_CARGO" "$@"
}

run_clippy_child() {
  local label="$1"
  shift
  ensure_frozen_offline_args "$@"
  run_child "$label" env CARGO="$ABS_CARGO" "$ABS_CARGO_CLIPPY" clippy "$@"
}

verify_evidence_ledger_honesty() {
  local doc="$ROOT/docs/security/vault-v2-verification.md"
  [ -f "$doc" ] || die "missing evidence ledger: $doc"
  local forbidden term
  forbidden="encrypted at rest
E2EE
recovery-complete
production-ready"
  while IFS= read -r term; do
    [ -z "$term" ] && continue
    if grep -qi "$term" "$doc"; then
      die "evidence ledger contains forbidden claim phrase: $term"
    fi
  done <<EOF
$forbidden
EOF
}

run_negative_env_smoke() {
  local probe="$1"
  local out status
  out=$(OPENSSL_DIR=/opt/attacker "$0" cargo metadata -p sovereign-vault-v2-engine --frozen --offline 2>&1) || status=$?
  status=${status:-0}
  if [ "$status" -eq 0 ]; then
    die "negative env smoke failed: wrapper accepted OPENSSL_DIR for $probe"
  fi
  note_step "negative-env-smoke"
}

run_full_qualification() {
  verify_evidence_ledger_honesty
  run_negative_env_smoke "full"
  run_cargo_child "tree-features" tree -p sovereign-vault-v2-engine -e features --frozen --offline
  run_cargo_child "build-engine" build -p sovereign-vault-v2-engine --frozen --offline
  run_cargo_child "test-engine" test -p sovereign-vault-v2-engine --frozen --offline
  run_clippy_child "clippy-engine" \
    -p sovereign-vault-v2-engine --all-targets --frozen --offline -- -D warnings
  {
    echo "## qualification steps"
    echo "$QUALIFY_STEPS"
    echo
    echo "## lock"
    echo "Cargo.lock sha256=$(tool_sha256 "$ROOT/Cargo.lock")"
  } >>"$MANIFEST"
  cat "$MANIFEST"
}

# ---- main -----------------------------------------------------------------
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT" || die "cannot cd to repository root"

if [ $# -eq 0 ]; then
  die "usage: $0 full | cargo <args...> (args must include --frozen and --offline)"
fi

QUALIFY_LOG="${QUALIFY_V2_LOG:-.harness/qualify-vault-v2.log}"
mkdir -p "$(dirname "$QUALIFY_LOG")"
: >"$QUALIFY_LOG"

reject_caller_shaping_env

REVIEWED_HOME="${HOME:-}"
REVIEWED_USERPROFILE="${USERPROFILE:-}"
REVIEWED_APPDATA="${APPDATA:-}"
REVIEWED_LOCALAPPDATA="${LOCALAPPDATA:-}"
REVIEWED_SYSTEMROOT="${SYSTEMROOT:-}"
REVIEWED_PATH="${PATH:-}"

REVIEWED_CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
if [ -n "${RUSTUP_HOME:-}" ]; then
  :
elif [ -d "$HOME/.rustup" ]; then
  RUSTUP_HOME="$HOME/.rustup"
fi

reject_cargo_configs
resolve_tools
verify_host_triple
build_child_path

FRESH_HOME=$(mktemp -d "${TMPDIR:-/tmp}/sovereign-qualify-home.XXXXXX")
FRESH_TMP=$(mktemp -d "${TMPDIR:-/tmp}/sovereign-qualify-tmp.XXXXXX")
TARGET_DIR=$(mktemp -d "${TMPDIR:-/tmp}/sovereign-qualify-target.XXXXXX")
MANIFEST=$(mktemp "${TMPDIR:-/tmp}/sovereign-qualify-manifest.XXXXXX")
emit_tool_manifest

MODE=$1
shift

case "$MODE" in
full)
  if [ $# -ne 0 ]; then
    die "full mode accepts no additional arguments"
  fi
  run_full_qualification
  ;;
cargo)
  if [ $# -eq 0 ]; then
    die "cargo mode requires arguments"
  fi
  reject_caller_config_args "$@"
  run_cargo_child "cargo-passthrough" "$@"
  ;;
*)
  die "unknown mode '$MODE' (only 'full' and 'cargo' are accepted)"
  ;;
esac

if [ -z "$(echo "$QUALIFY_STEPS" | tr -d '[:space:]')" ]; then
  die "no qualification steps ran — refusing to exit successfully"
fi

QUALIFY_STATE=completed
echo "qualify-vault-v2: OK steps:${QUALIFY_STEPS}"
