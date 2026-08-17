#!/bin/zsh

set -euo pipefail

root_dir="${0:A:h:h}"
default_key_path="$HOME/.codex/secrets/ai-quota-trends-updater.key"
default_password_path="$HOME/.codex/secrets/ai-quota-trends-updater.password"
legacy_key_path="$HOME/.codex/secrets/codex-quota-trends-updater.key"
legacy_password_path="$HOME/.codex/secrets/codex-quota-trends-updater.password"

if [[ ! -e "$default_key_path" && -r "$legacy_key_path" ]]; then
  default_key_path="$legacy_key_path"
fi
if [[ ! -e "$default_password_path" && -r "$legacy_password_path" ]]; then
  default_password_path="$legacy_password_path"
fi

key_path="${TAURI_SIGNING_PRIVATE_KEY_FILE:-$default_key_path}"
password_path="${TAURI_SIGNING_PRIVATE_KEY_PASSWORD_FILE:-$default_password_path}"

if [[ ! -r "$key_path" || ! -r "$password_path" ]]; then
  print -u2 "Missing updater signing key or password file."
  print -u2 "Expected: $key_path"
  print -u2 "Expected: $password_path"
  exit 1
fi

version="$(node -p "require('$root_dir/apps/gui/src-tauri/tauri.conf.json').version")"
package_version="$(node -p "require('$root_dir/apps/gui/package.json').version")"
workspace_version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$root_dir/Cargo.toml" | head -1)"

if [[ "$version" != "$package_version" || "$version" != "$workspace_version" ]]; then
  print -u2 "Release versions must match before building."
  print -u2 "Tauri: $version, package: $package_version, workspace: $workspace_version"
  exit 1
fi

export CARGO_TARGET_DIR="$root_dir/target.noindex"
bundle_dir="$CARGO_TARGET_DIR/universal-apple-darwin/release/bundle"
archive="$bundle_dir/macos/AI Quota Trends.app.tar.gz"
signature="$archive.sig"
manifest="$bundle_dir/macos/latest.json"

export TAURI_SIGNING_PRIVATE_KEY="$(< "$key_path")"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(< "$password_path")"

cd "$root_dir/apps/gui"
npm run tauri build -- \
  --target universal-apple-darwin \
  --bundles app,dmg \
  --config '{"bundle":{"macOS":{"signingIdentity":"-"}}}'

cd "$root_dir"
node scripts/generate-update-manifest.mjs "$version" "$archive" "$signature" "$manifest"

print "Local release artifacts:"
print "  $bundle_dir/macos/AI Quota Trends.app"
print "  $archive"
print "  $signature"
print "  $bundle_dir/dmg/AI Quota Trends_${version}_universal.dmg"
print "  $manifest"
