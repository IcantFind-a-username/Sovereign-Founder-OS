# Path-to-package mapping for the scoped quality gate.
# Sourced by scripts/test_changed.sh and scripts/tests/gate_portability_test.sh.
# Never executed on its own; never exits 0 as a success signal.
#
# Portability: bash 3.2 (stock macOS) — no associative arrays, no `mapfile`,
# no case-modifying expansions. PKGS is a space-delimited, space-padded list
# of cargo package names for that reason.
#
# Path resolution for #[path = "…"] uses `cd "$(dirname …)" && pwd -P`
# (no `realpath -m`, which bash 3.2 / macOS do not ship).

add_pkg() {
  case " $PKGS " in
  *" $1 "*) ;;
  *) PKGS="$PKGS $1" ;;
  esac
}

map_one_changed_path() { # <repo-relative path>
  local f=$1 rest
  case "$f" in
  Cargo.toml | Cargo.lock | rust-toolchain.toml | scripts/* | .github/*)
    FULL=1
    ;;
  crates/*/*)
    rest=${f#crates/}
    add_pkg "sovereign-${rest%%/*}"
    ;;
  apps/cli/assets/*)
    FRONTEND=1
    ;;
  apps/cli/*)
    add_pkg "sovereign-cli"
    ;;
  tests/adversarial/*)
    add_pkg "sovereign-adversarial-tests"
    ;;
  esac
}

# Map CHANGED (newline-delimited repo-relative paths) onto FULL / FRONTEND / PKGS.
map_changed_paths_to_scope() {
  FULL=0
  FRONTEND=0
  PKGS=""
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    map_one_changed_path "$f"
  done <<EOF
$CHANGED
EOF
# ---- resolve #[path] includes ------------------------------------------
  map_path_attr_includes || return 1
}

_extract_rust_path_attrs() { # <file> -> relative path strings, one per line
  sed -n 's/.*#\[path[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$1"
}

# Resolve #[path = "rel"] against the including file. bash 3.2: cd + pwd -P,
# never realpath -m. Prints a repo-relative target or returns nonzero.
_resolve_rust_path_attr() { # <repo_root> <includer> <rel>
  local repo_root includer rel includer_dir target_dir abs
  repo_root=$1
  includer=$2
  rel=$3
  [ -n "$rel" ] || return 1
  includer_dir=$(cd "$repo_root" && cd "$(dirname "$includer")" && pwd -P) || return 1
  target_dir=$(cd "$includer_dir" && cd "$(dirname "$rel")" && pwd -P) || return 1
  abs="$target_dir/$(basename "$rel")"
  case "$abs" in
  "$repo_root"/*)
    printf '%s\n' "${abs#"$repo_root"/}"
    ;;
  *)
    return 1
    ;;
  esac
}

_changed_has_path() { # <repo-relative path>
  case "
$CHANGED
" in
  *"
$1
"*) return 0 ;;
  *) return 1 ;;
  esac
}

# When a changed file is the target of a #[path = "…"] include, also queue
# the including file's package. An included file has no Cargo edge to its
# includer, so the crate-prefix rule above cannot see it. Scans tracked and
# untracked Rust files; string-literal false positives are conservative.
map_path_attr_includes() {
  local repo_root rs rel target rs_files
  [ -n "$CHANGED" ] || return 0
  [ "$FULL" -eq 1 ] && return 0
  repo_root=$(cd "$(git rev-parse --show-toplevel)" && pwd -P) || return 1
  rs_files=$(
    git -C "$repo_root" ls-files -- '*.rs' || exit 1
    git -C "$repo_root" ls-files --others --exclude-standard -- '*.rs' || exit 1
  ) || return 1
  while IFS= read -r rs; do
    [ -n "$rs" ] || continue
    [ -f "$repo_root/$rs" ] || continue
    while IFS= read -r rel; do
      [ -n "$rel" ] || continue
      target=$(_resolve_rust_path_attr "$repo_root" "$rs" "$rel") || continue
      [ -n "$target" ] || continue
      if _changed_has_path "$target"; then
        map_one_changed_path "$rs"
      fi
    done <<EOF
$(_extract_rust_path_attrs "$repo_root/$rs")
EOF
  done <<EOF
$rs_files
EOF
}
