#!/usr/bin/env bash
# Compila o SDL2 2.32.10 (apenas áudio) a partir do submodule vendor/SDL e
# prepara `runtime/` com as bibliotecas que precisam acompanhar o instalador:
#
#   MesenCore.dylib / .so   (núcleo do MesenCE)
#   libMesenCore.dylib / .so (symlink usado por -lMesenCore)
#   libSDL2-2.0.0.dylib / libSDL2-2.0.so.0
#   LICENSE-SDL2.txt
#
# O Mesen NÃO linka com SDL2: `Sdl/include/Sdl2.h` chama dlopen() com nomes
# curtos. Por isso o núcleo recebe um rpath apontando para o próprio diretório
# (`@loader_path` no macOS, `$ORIGIN` no Linux) e as bibliotecas ficam juntas.
set -euo pipefail

CRATE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="$CRATE_DIR/vendor/SDL"
OUT="$CRATE_DIR/runtime"
# Alias usado apenas pelo linker (-lMesenCore). Fica FORA de `runtime/` para
# que o dyld não carregue duas cópias do núcleo (MesenCore + libMesenCore) —
# isso travava o Emulator::Stop ao encerrar.
LINK_OUT="$CRATE_DIR/runtime-link"
TARGET="${MESEN_TARGET:-$(rustc -vV | sed -n 's/^host: //p')}"

case "$TARGET" in
  x86_64-apple-darwin|aarch64-apple-darwin)
    PLATFORM=macos
    SDL_LIB=libSDL2-2.0.0.dylib
    CORE_FILE=MesenCore.dylib
    DEFAULT_MESEN_DIR=/tmp/MesenCE/InteropDLL/obj.osx-arm64
    ;;
  x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu)
    PLATFORM=linux
    SDL_LIB=libSDL2-2.0.so.0
    CORE_FILE=MesenCore.so
    DEFAULT_MESEN_DIR=/tmp/MesenCE/InteropDLL/obj.linux-x64
    ;;
  *)
    echo "Target sem suporte: $TARGET (use x86_64/aarch64 apple-darwin ou unknown-linux-gnu)" >&2
    exit 1
    ;;
esac

if [[ ! -f "$SRC/configure" || ! -f "$SRC/LICENSE.txt" ]]; then
  echo "Submodule SDL ausente. Rode: git submodule update --init --recursive" >&2
  exit 1
fi

MESEN_LIB_DIR="${MESEN_LIB_DIR:-$DEFAULT_MESEN_DIR}"
if [[ ! -f "$MESEN_LIB_DIR/$CORE_FILE" ]]; then
  echo "Núcleo do Mesen não encontrado em $MESEN_LIB_DIR/$CORE_FILE." >&2
  echo "Defina MESEN_LIB_DIR com o diretório do MesenCore compilado." >&2
  exit 1
fi

TMP="$(mktemp -d "${TMPDIR:-/tmp}/emmu-sdl2.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/src"
cp -R "$SRC/." "$TMP/src/"
cd "$TMP/src"

# O Emmu usa somente as 6 funções de áudio de Sdl2.h. Desligar vídeo,
# renderização, joysticks e drivers de áudio não usados mantém o binário
# pequeno e sem dependências de X11/Wayland/PulseAudio na distribuição.
CONFIGURE_FLAGS=(
  --enable-shared --disable-static
  --disable-video --disable-render --disable-events --disable-joystick
  --disable-haptic --disable-hidapi --disable-sensor --disable-power
  --disable-filesystem --disable-diskaudio
  --disable-alsa --disable-pulseaudio --disable-pipewire --disable-jack
  --disable-esd --disable-arts --disable-nas --disable-sndio
  --disable-video-x11 --disable-video-wayland --disable-video-cocoa
  --disable-video-rpi --disable-video-offscreen --disable-video-kmsdrm
)

JOBS="$( (sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo 2) | head -1 )"
if [[ "$PLATFORM" == "macos" ]]; then
  # Universal: serve Intel e Apple Silicon com o mesmo instalador.
  export CFLAGS="${CFLAGS:-} -O2 -arch arm64 -arch x86_64"
  export LDFLAGS="${LDFLAGS:-} -arch arm64 -arch x86_64"
elif [[ "$TARGET" == aarch64-* ]]; then
  export CFLAGS="${CFLAGS:-} -O2"
fi

echo "==> Configurando SDL2 $(git -C "$SRC" describe --tags --always) para $TARGET"
./configure "${CONFIGURE_FLAGS[@]}" > "$TMP/configure.log" 2>&1 || {
  tail -40 "$TMP/configure.log" >&2
  exit 1
}
make -j"$JOBS" > "$TMP/build.log" 2>&1 || {
  tail -40 "$TMP/build.log" >&2
  exit 1
}

mkdir -p "$OUT" "$LINK_OUT"
cp "build/.libs/$SDL_LIB" "$OUT/$SDL_LIB"
cp "$SRC/LICENSE.txt" "$OUT/LICENSE-SDL2.txt"

if [[ "$PLATFORM" == "macos" ]]; then
  install_name_tool -id "@rpath/$SDL_LIB" "$OUT/$SDL_LIB"
fi

# Copia o núcleo e cria o alias usado pelo -lMesenCore do build.rs em outro
# diretório (ver comentário no topo do arquivo).
cp "$MESEN_LIB_DIR/$CORE_FILE" "$OUT/$CORE_FILE"
ln -sfn "../runtime/$CORE_FILE" "$LINK_OUT/lib$CORE_FILE"

# Faz o dlopen() de nome curto dentro do Mesen resolver a cópia empacotada.
if [[ "$PLATFORM" == "macos" ]]; then
  install_name_tool -delete_rpath @loader_path "$OUT/$CORE_FILE" 2>/dev/null || true
  install_name_tool -add_rpath @loader_path "$OUT/$CORE_FILE"
elif command -v patchelf >/dev/null 2>&1; then
  patchelf --set-rpath '$ORIGIN' "$OUT/$CORE_FILE"
else
  echo "Aviso: patchelf não encontrado; defina LD_LIBRARY_PATH no instalador Linux." >&2
fi

echo "==> Pronto: $OUT"
ls -lh "$OUT"
