#!/usr/bin/env bash
# Launches a built Linux bundle the way a person would, and fails if it
# does not start. Nothing else in the build ever runs what it packages:
# a library left out of lib/, a dependency the .deb forgot to name, or an
# instruction an older CPU lacks all used to reach a release unnoticed.
#
#   smoke-linux.sh [--sde SDE64] DIR
#       The staged folder (DIR/concat beside DIR/lib), on this machine:
#       every library resolves from the folder alone; `concat --version`
#       exits 0; `concat --smoke` (start-up through the first frame of the
#       window, then out) exits 0 under Xvfb, and does not so much as try
#       to open the NVIDIA driver's CUDA or NVENC libraries, which nothing
#       should touch before a person asks for a hardware export. With
#       --sde, `--version` again under Intel's emulator posing as an Ivy
#       Bridge (AVX, no AVX2), the oldest CPU the x86_64 bundle promises,
#       and `--whisper-cpu`, loading whisper's CPU kernels, posing as a
#       Sandy Bridge.
#
#   smoke-linux.sh --container IMAGE DEB APPIMAGE
#       The packages, in a clean IMAGE (ubuntu:22.04, debian:12) through
#       docker: the .deb installed by apt, which pulls in exactly the
#       dependencies it declares and nothing else, then `--version` and
#       `--smoke` from the installed package and from the AppImage.
#
#   smoke-linux.sh --container-rpm IMAGE RPM
#       The .rpm, in a clean IMAGE (fedora:latest) through docker: installed
#       by dnf, which pulls in exactly the dependencies it declares and
#       nothing else, then `--version` and `--smoke` from the installed
#       package. The .rpm names its dependencies as RPM packages, and only
#       an RPM tool can say whether those names are ones that exist.
#
#   smoke-linux.sh --container-pacman IMAGE PKG
#       The Arch package, in a clean IMAGE (archlinux:latest) through
#       docker: installed by pacman, then `--version` and `--smoke` from
#       the installed package.
#
# The .deb and the AppImage were the only packages ever installed by a
# test: a .rpm whose dependencies name packages RPM does not have, or an
# Arch package whose depends list is short, reached every person who
# installed it and nobody else.
#
# Runs in CI after staging (build-app.yml), and locally against a folder
# or packages built the same way. Needs xvfb-run; strace if it is there.
set -euo pipefail

here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)

say() { printf '\n== %s\n' "$*"; }
fail() {
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::error::$*"; else echo "smoke: $*" >&2; fi
  exit 1
}

# A fresh home for each run, so the start-up is a first start-up: no
# settings, no recent projects, nothing cached from a run before.
fresh_home() {
  local home
  home=$(mktemp -d)
  mkdir -p "$home/.config" "$home/.cache" "$home/.local/share"
  echo "$home"
}

# Runs a command with a time limit, prints what it wrote, and fails with
# what it is told when the command does not exit 0.
expect_ok() {
  local what=$1 secs=$2 out status=0
  shift 2
  out=$(timeout --kill-after=10 "$secs" "$@" 2>&1) || status=$?
  printf '%s\n' "$out" | tail -n 60
  if [ "$status" -eq 124 ] || [ "$status" -eq 137 ]; then
    fail "$what: still running after ${secs}s"
  elif [ "$status" -ne 0 ]; then
    fail "$what: exited with $status"
  fi
}

host() {
  local sde="" dir
  if [ "${1:-}" = "--sde" ]; then sde=$2; shift 2; fi
  [ $# -eq 1 ] || { echo "usage: $0 [--sde SDE64] DIR" >&2; exit 2; }
  dir=$(cd "$1" && pwd)
  local bin="$dir/concat"
  [ -x "$bin" ] || fail "$bin is not there"

  # The build's own environment points at the FFmpeg it linked; the
  # folder has to stand without it.
  unset LD_LIBRARY_PATH

  say "every library resolves from the folder"
  local report
  report=$(ldd "$bin" || true)
  if grep -i 'not found' <<< "$report"; then
    echo "$report"
    fail "the binary does not resolve every library from its own folder"
  fi

  say "concat --version"
  HOME=$(fresh_home) expect_ok "concat --version" 60 "$bin" --version

  say "concat --smoke"
  local trace
  trace=$(mktemp)
  local tracer=()
  if command -v strace >/dev/null 2>&1; then
    tracer=(strace -f -qq -e "trace=open,openat" -o "$trace")
  else
    echo "no strace: the start-up is run but its file opens are not checked"
  fi
  HOME=$(fresh_home) expect_ok "concat --smoke" 180 \
    xvfb-run -a -s "-screen 0 1920x1080x24" "${tracer[@]}" "$bin" --smoke
  if [ ${#tracer[@]} -gt 0 ]; then
    local cuda
    cuda=$(grep -E 'lib(cuda|nvcuvid|nvidia-encode)\.so' "$trace" || true)
    if [ -n "$cuda" ]; then
      echo "$cuda"
      fail "concat --smoke tried to load the NVIDIA driver's CUDA or NVENC libraries at start-up"
    fi
    echo "no CUDA or NVENC library opened at start-up"
  fi

  if [ -n "$sde" ]; then
    say "concat --version on an emulated Ivy Bridge"
    # The emulator stops the program at the first instruction the chip
    # it poses as does not have, and says which.
    HOME=$(fresh_home) expect_ok "concat --version under SDE -ivb" 600 "$sde" -ivb -- "$bin" --version

    # whisper's CPU kernels, one module per CPU generation: ggml opens
    # every one to score it against the CPU before it keeps the best, so
    # the Haswell and Ice Lake modules are loaded here too, on a Sandy
    # Bridge - AVX without even F16C, like #318's i7-2600 - which must
    # come out with the sandybridge module and no illegal instruction.
    say "concat --whisper-cpu on an emulated Sandy Bridge"
    HOME=$(fresh_home) expect_ok "concat --whisper-cpu under SDE -snb" 600 "$sde" -snb -- "$bin" --whisper-cpu
  fi
}

container() {
  [ $# -eq 3 ] || { echo "usage: $0 --container IMAGE DEB APPIMAGE" >&2; exit 2; }
  local image=$1 deb appimage
  deb=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
  appimage=$(cd "$(dirname "$3")" && pwd)/$(basename "$3")
  [ -f "$deb" ] || fail "$deb is not there"
  [ -f "$appimage" ] || fail "$appimage is not there"
  say "in a clean $image"
  docker run --rm \
    -v "$here/smoke-linux.sh:/ci/smoke-linux.sh:ro" \
    -v "$deb:/pkg/concat.deb:ro" \
    -v "$appimage:/pkg/concat.AppImage:ro" \
    "$image" bash /ci/smoke-linux.sh --inside /pkg/concat.deb /pkg/concat.AppImage
}

container_rpm() {
  [ $# -eq 2 ] || { echo "usage: $0 --container-rpm IMAGE RPM" >&2; exit 2; }
  local image=$1 rpm
  rpm=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
  [ -f "$rpm" ] || fail "$rpm is not there"
  say "in a clean $image"
  docker run --rm \
    -v "$here/smoke-linux.sh:/ci/smoke-linux.sh:ro" \
    -v "$rpm:/pkg/concat.rpm:ro" \
    "$image" bash /ci/smoke-linux.sh --inside-rpm /pkg/concat.rpm
}

container_pacman() {
  [ $# -eq 2 ] || { echo "usage: $0 --container-pacman IMAGE PKG" >&2; exit 2; }
  local image=$1 pkg
  pkg=$(cd "$(dirname "$2")" && pwd)/$(basename "$2")
  [ -f "$pkg" ] || fail "$pkg is not there"
  say "in a clean $image"
  docker run --rm \
    -v "$here/smoke-linux.sh:/ci/smoke-linux.sh:ro" \
    -v "$pkg:/pkg/concat.pkg.tar.zst:ro" \
    "$image" bash /ci/smoke-linux.sh --inside-pacman /pkg/concat.pkg.tar.zst
}

# Inside the container. Xvfb and Mesa's software drivers stand in for
# the desktop's display and GPU, and one font for the desktop's fonts;
# everything else the program needs comes from the package's own
# dependencies, so a library it forgot to name is missing here too.
inside() {
  local deb=$1 appimage=$2
  export DEBIAN_FRONTEND=noninteractive
  say "installing the .deb and what it depends on"
  apt-get update -qq
  apt-get install -y -qq --no-install-recommends \
    "$deb" xvfb xauth libgl1-mesa-dri mesa-vulkan-drivers fonts-dejavu-core > /dev/null

  say "concat --version (the package)"
  HOME=$(fresh_home) expect_ok "concat --version from the .deb" 60 concat --version
  say "concat --smoke (the package)"
  HOME=$(fresh_home) expect_ok "concat --smoke from the .deb" 180 \
    xvfb-run -a -s "-screen 0 1920x1080x24" concat --smoke

  # No FUSE in a container: the AppImage unpacks itself and runs.
  export APPIMAGE_EXTRACT_AND_RUN=1
  say "concat --version (the AppImage)"
  HOME=$(fresh_home) expect_ok "concat --version from the AppImage" 120 "$appimage" --version
  say "concat --smoke (the AppImage)"
  HOME=$(fresh_home) expect_ok "concat --smoke from the AppImage" 240 \
    xvfb-run -a -s "-screen 0 1920x1080x24" "$appimage" --smoke
}

# Inside a Fedora container: the .rpm, installed by dnf. A dependency the
# package names as an RPM package that RPM does not have fails here.
inside_rpm() {
  local rpm=$1
  say "installing the .rpm and what it depends on"
  dnf install -y "$rpm" \
    xorg-x11-server-Xvfb xorg-x11-xauth mesa-vulkan-drivers dejavu-fonts > /dev/null

  say "concat --version (the package)"
  HOME=$(fresh_home) expect_ok "concat --version from the .rpm" 60 concat --version
  say "concat --smoke (the package)"
  HOME=$(fresh_home) expect_ok "concat --smoke from the .rpm" 180 \
    xvfb-run -a -s "-screen 0 1920x1080x24" concat --smoke
}

# Inside an Arch container: the package, installed by pacman.
inside_pacman() {
  local pkg=$1
  say "installing the package and what it depends on"
  # The package database first: a container has none, and pacman needs
  # one to resolve what the package depends on.
  pacman -Sy --noconfirm > /dev/null
  pacman -S --noconfirm --needed \
    xorg-server xorg-xauth mesa vulkan-icd-loader ttf-dejavu > /dev/null
  pacman -U --noconfirm "$pkg" > /dev/null

  say "concat --version (the package)"
  HOME=$(fresh_home) expect_ok "concat --version from the package" 60 concat --version
  say "concat --smoke (the package)"
  HOME=$(fresh_home) expect_ok "concat --smoke from the package" 180 \
    xvfb-run -a -s "-screen 0 1920x1080x24" concat --smoke
}

case "${1:-}" in
  --container) shift; container "$@" ;;
  --container-rpm) shift; container_rpm "$@" ;;
  --container-pacman) shift; container_pacman "$@" ;;
  --inside) shift; inside "$@" ;;
  --inside-rpm) shift; inside_rpm "$@" ;;
  --inside-pacman) shift; inside_pacman "$@" ;;
  *) host "$@" ;;
esac
say "ok"
