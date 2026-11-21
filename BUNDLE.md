# SDL2 no pacote de distribuição

O MesenCE **não linka** com SDL2. `Sdl/include/Sdl2.h` carrega a biblioteca em
runtime com `dlopen()` e apenas 6 funções de áudio (`SDL_InitSubSystem`,
`SDL_OpenAudioDevice`, `SDL_CloseAudioDevice`, `SDL_GetNumAudioDevices`,
`SDL_GetAudioDeviceName`, `SDL_PauseAudioDevice`). Isso foi confirmado com
`otool -L libMesenCore.dylib`: nenhuma entrada SDL2.

Portanto, para distribuir o Emmu sem exigir `brew install sdl2` ou
`apt install libsdl2`, basta levar a biblioteca junto e fazer o `dlopen` de
nome curto encontrá-la.

## Versão

SDL2 **2.32.10** (tag `release-2.32.10` do
[libsdl-org/SDL](https://github.com/libsdl-org/SDL)), licença zlib. Foi
escolhido o SDL2 clássico em vez do `sdl2-compat` do Homebrew porque este é um
shim sobre SDL3 e puxaria uma segunda biblioteca. O submodule fica em
`vendor/SDL`, fixado nessa tag.

## Como gerar

```sh
git submodule update --init --recursive
# macOS (aceita MESEN_LIB_DIR para apontar outro build do MesenCE):
./crates/emmu-mesen-sys/scripts/build-sdl2.sh
# Linux x86_64:
MESEN_TARGET=x86_64-unknown-linux-gnu MESEN_LIB_DIR=/caminho/do/MesenCE \
  ./crates/emmu-mesen-sys/scripts/build-sdl2.sh
```

O script:

1. compila o SDL2 de `vendor/SDL` **somente com áudio** (desliga vídeo, render,
   joystick, X11/Wayland, PulseAudio etc. — o resultado é ~1,9 MB);
2. no macOS gera um binário universal (arm64 + x86_64) e ajusta o install name
   para `@rpath/libSDL2-2.0.0.dylib`;
3. copia `MesenCore` e `libSDL2` para `crates/emmu-mesen-sys/runtime/` e deixa
   em `runtime-link/` apenas o symlink usado pelo `-lMesenCore` do linker
   (separado porque dois nomes para o mesmo dylib no mesmo diretório fazem o
   dyld carregar o núcleo duas vezes e travam o `Emulator::Stop`);
4. adiciona `@loader_path` (macOS) / `$ORIGIN` (Linux, via `patchelf`) ao
   `MesenCore` — é isso que faz `dlopen("libSDL2-2.0.0.dylib")` resolver a
   cópia empacotada;
5. copia `LICENSE-SDL2.txt`.

## Como entra no instalador

`platforms/desktop/tauri.conf.json` declara
`"../../crates/emmu-mesen-sys/runtime/*"` em `bundle.resources`, então os
arquivos vão para `$RESOURCE` (`Contents/Resources/` no macOS). `runtime-link/`
não entra no instalador. O
`platforms/desktop/build.rs` adiciona o rpath correspondente ao binário
(`@executable_path/../Resources` no macOS, `$ORIGIN/../lib` no Linux), e o
`build.rs` deste crate passa a linkar a cópia empacotada quando ela existe.
Se `runtime/` não existir, o build volta ao comportamento antigo
(`MESEN_LIB_DIR` ou `/tmp/MesenCE/...`) para não quebrar o desenvolvimento.

Para desenvolvimento (`cargo tauri dev`) nada muda: o Mesen e o SDL2 do sistema
continuam sendo usados se `runtime/` não tiver sido gerado.
