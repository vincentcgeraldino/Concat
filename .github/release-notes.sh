#!/usr/bin/env bash
# The notes for a release.
#
# Usage: release-notes.sh <since-ref> <until-ref> <version>
#
# Writes markdown to stdout: what changed, what to know before installing,
# then how to download each bundle and the checksum file.
#
# "Before you install" says what the pipeline could not vouch for, so a
# release never reads as more checked than it was. The caller says which:
# WINDOWS_SIGNED=false when no certificate signed the Windows bundles,
# MACOS_NOTARISED=false when Apple's notary did not see the macOS disk
# images, ANDROID_STARTED=false when no launch check got as far as the
# APK's own code, and ANDROID_KEYED=false when the APK kept the key made
# for the build instead of the repository's.
#
# What changed is the subjects of the commits between the two refs,
# filtered as follows.
#
# What a release note is for is what somebody running the app can notice, so
# three kinds of subject are dropped on the way past. Housekeeping, by its
# shape: formatting, lock files, "Update foo.rs". Anything typed as work on
# the tree rather than on the product - a conventional-commit `test:`,
# `chore:`, `ci:`, `build:`, `refactor:`, `style:` or `docs:` says so
# itself, and so does a fix scoped to the pipeline, `fix(release):` or
# `fix(ci):` - a smoke script that can run is not something a person who
# downloads the app can notice. And a fix that only got the build green
# again: a lint appeased, an import removed, a file moved to please a
# compiler is not a fix anybody asked for.
#
# A subject that survives is printed as a sentence. A `feat(studio):` in
# front of one is scaffolding for the log and reads as noise in a release,
# so the type comes off and the first letter goes up. A subject that appears
# twice appears once.
set -euo pipefail

since="$1"
until="$2"
version="$3"
title="Concat $version"

# A conventional-commit type, with its optional (scope) and breaking `!`.
type='^[a-z0-9]+(\([^)]*\))?!?: '

# Each line opens with one mark for what it touches, from the commit's
# scope where it has one and its type where it has not - a fix reads as a
# fix at a glance, and the text rows stand apart from the timeline's. A
# subject with neither carries no mark rather than a wrong one.
marks='text=📝 speech=🗣️ enhance=🗣️ keyframes=🎞️ effects=✨ render=✨ colour=✨ color=✨ timeline=✂️ media=📁 export=📤 settings=⚙️ i18n=🌍 locales=🌍 api=🤖 server=🤖 android=📱 ios=📱 models=📦 inspector=🖥️ window=🖥️ workspace=🖥️ ui=🖥️ start=🖥️ feat=✨ fix=🐛 perf=⚡'
changes=$(
  # Each filter prints nothing when it matches nothing, and `grep` says so
  # with a status of 1. Under `pipefail` that is the pipeline's status, and
  # the assignment below failed - which took the whole script with it, and
  # with it the release: a version bump on its own, or a range of nothing
  # but filtered-out commits, left no notes at all and no release either.
  { git log "$since..$until" --no-merges --format='%s' 2>/dev/null || true; } \
  | { grep -Ev '^((Create|Update) [^ ]+\.[a-z]+|Version [0-9]|Lock the flake|Format the workspace|Changelog for|Merge )' || true; } \
  | { grep -Ev 'in the (export|pool) tests$' || true; } \
  | { grep -Eiv '^(test|chore|ci|build|refactor|style|docs)(\([^)]*\))?!?: ' || true; } \
  | { grep -Eiv '^[a-z0-9]+\((release|ci)\)!?: ' || true; } \
  | { grep -Eiv "${type}.*(clippy|lint|rustfmt|(unused|duplicate|missing) [A-Za-z]* ?import|non-existent|does not compile|before test module|green again)" || true; } \
  | { grep -Eiv '^(updates?|wip|fixes?|cleanup)\.?$' || true; } \
  | awk -v marks="$marks" '
      BEGIN {
        n = split(marks, pairs, " ")
        for (i = 1; i <= n; i++) { split(pairs[i], kv, "="); mark[kv[1]] = kv[2] }
      }
      {
        prefix = ""
        if (match($0, /^[a-z0-9]+(\([^)]*\))?!?: /)) {
          head = substr($0, 1, RLENGTH)
          $0 = substr($0, RLENGTH + 1)
          kind = head; sub(/[(!:].*/, "", kind)
          scope = ""
          if (match(head, /\([^)]*\)/)) scope = substr(head, RSTART + 1, RLENGTH - 2)
          if (scope in mark) prefix = mark[scope] " "
          else if (kind in mark) prefix = mark[kind] " "
        }
        print prefix toupper(substr($0, 1, 1)) substr($0, 2)
      }' \
  | awk '!seen[$0]++' \
  | sed 's/^/- /')

echo "## $title"
echo
if [ -n "$changes" ]; then
  echo "### What changed"
  echo
  echo "$changes"
  echo
fi
caveats=()
if [ "${WINDOWS_SIGNED:-true}" = false ]; then
  caveats+=("- **Windows:** this release is not code-signed yet. SmartScreen asks before it runs: **More info** › **Run anyway**. Where Smart App Control is on, Windows refuses unsigned programs; see the [code signing policy](https://github.com/jub0t/concat/blob/main/CODE_SIGNING.md).")
fi
if [ "${ANDROID_STARTED:-true}" = false ]; then
  caveats+=("- **Android:** no automated check has started this APK: CI's emulators run x86_64 and it is built for arm64. If it does not open on your phone, [say so](https://github.com/jub0t/concat/issues).")
fi
if [ "${ANDROID_KEYED:-true}" = false ]; then
  caveats+=("- **Android:** this APK is signed with a key made for the build, not the repository's. It installs, but a later release signed with the repository's key cannot install over it: uninstall this one first, and your app data goes with it.")
fi
if [ "${MACOS_NOTARISED:-true}" = false ]; then
  caveats+=("- **macOS:** Apple did not notarise this release, so Gatekeeper will not open it on its own. Drag Concat to your Applications folder, then hold **Control** and click it and choose **Open**. A notarised release opens on its own.")
fi
if [ "${#caveats[@]}" -gt 0 ]; then
  echo "### Before you install"
  echo
  printf '%s\n' "${caveats[@]}"
  echo
fi
# The names the files actually have, as `scripts/models.py` lists them:
# `Concat-<version>-<platform>-<arch>.<kind>`. They used to be written
# without the prefix or the version, and so named files that are not on
# the release page at all.
cat <<'MD' | sed "s/@VERSION@/$version/g"
### Download

A self-contained build for every platform Concat ships on.

| | Apple silicon | Intel / x86_64 | arm64 |
|---|---|---|---|
| macOS | `Concat-@VERSION@-macos-arm64.dmg` | `Concat-@VERSION@-macos-x86_64.dmg` | |
| Windows | | `Concat-@VERSION@-windows-x86_64-setup.exe`, `Concat-@VERSION@-windows-x86_64.msi` | |
| Linux | | `Concat-@VERSION@-linux-x86_64.deb`, `.rpm`, `Concat-@VERSION@-x86_64.AppImage`, `Concat-@VERSION@-linux-x86_64.pkg.tar.zst` | `Concat-@VERSION@-linux-aarch64.deb`, `.rpm`, `Concat-@VERSION@-aarch64.AppImage`, `Concat-@VERSION@-linux-aarch64.pkg.tar.zst` |
| Android | | | `Concat-@VERSION@-android-arm64.apk` |
| iOS / iPadOS | | | `Concat-@VERSION@-ios-arm64.ipa` |

`SHA256SUMS` lists each file's checksum.
MD
