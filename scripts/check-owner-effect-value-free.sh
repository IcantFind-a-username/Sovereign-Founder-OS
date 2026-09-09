#!/usr/bin/env bash
# No fixture type may print its own secrets.
#
# Several types in this work carry key material: a launch key, a connection
# key, a session cookie and its CSRF partner. Each has a hand-written `Debug`
# that prints `<redacted>`, and each has a test proving it. That is the state
# today and it is not what keeps it true — the next struct to hold a key will
# be written by someone who reaches for `#[derive(Debug)]` because every other
# struct in the file has one, and the first time anyone notices is when a key
# appears in a log.
#
# So the rule is checked structurally: a struct in the fixture's own modules
# that holds a secret-shaped field must not derive `Debug`. It must write one,
# where the choice about what to print is deliberate and visible in review.
#
# This is a lint, not a proof. It cannot see a secret stored under a name it
# does not recognise, and it is not trying to — it removes the failure that
# happens by habit, which is the one that actually happens.
#
# Portability: bash 3.2 (stock macOS).
set -euo pipefail

if [ -z "${BASH_VERSINFO:-}" ] || [ "${BASH_VERSINFO[0]}" -lt 3 ] ||
  { [ "${BASH_VERSINFO[0]}" -eq 3 ] && [ "${BASH_VERSINFO[1]}" -lt 2 ]; }; then
  echo "unsupported bash: need 3.2 or newer, got ${BASH_VERSION:-non-bash shell}" >&2
  exit 3
fi

SCANNED="crates/authority/src/broker
crates/owner/src"

# Field names that mean "this is secret". Deliberately short: a longer list
# invites the belief that it is exhaustive.
SECRET_FIELDS="key,cookie,csrf,launch_key,secret,token,password"

completed=0
on_exit() {
  status=$?
  if [ "$completed" -eq 0 ] && [ "$status" -eq 0 ]; then
    echo "check-owner-effect-value-free: INCOMPLETE — exited before finishing" >&2
    exit 1
  fi
}
trap on_exit EXIT

fail=0
files=0

for directory in $SCANNED; do
  [ -d "$directory" ] || continue
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    files=$((files + 1))
    # Walk the file, tracking whether the struct being declared derives Debug
    # and whether it holds a secret-shaped field.
    awk -v secrets="$SECRET_FIELDS" -v path="$file" '
      BEGIN {
        n = split(secrets, list, ",")
        for (i = 1; i <= n; i++) if (list[i] != "") secret[list[i]] = 1
      }
      /^#\[derive\(/ { derives = ($0 ~ /Debug/); next }
      /^(pub )?struct / {
        name = $0
        sub(/^(pub )?struct /, "", name)
        sub(/[ ({<].*$/, "", name)
        current = name
        current_derives = derives
        derives = 0
        next
      }
      # A field line inside the struct being tracked.
      current != "" && /^ +(pub )?[a-z_]+:/ {
        field = $0
        sub(/^ +(pub )?/, "", field)
        sub(/:.*$/, "", field)
        if (current_derives && (field in secret)) {
          printf "FAIL  %s: struct %s derives Debug and holds `%s`\n", path, current, field
          bad = 1
          current = ""
        }
        next
      }
      /^}/ { current = ""; derives = 0 }
      END { exit bad ? 1 : 0 }
    ' "$file" || fail=1
  done <<FILES_EOF
$(find "$directory" -name '*.rs' -type f)
FILES_EOF
done

if [ "$files" -eq 0 ]; then
  echo "check-owner-effect-value-free: scanned zero files" >&2
  exit 1
fi

echo "ok    $files fixture source files scanned for derived Debug on secrets"
if [ "$fail" -ne 0 ]; then
  echo "check-owner-effect-value-free: FAILED" >&2
  exit 1
fi

completed=1
echo "check-owner-effect-value-free: OK"
