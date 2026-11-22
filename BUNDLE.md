# Dependências de runtime no pacote de distribuição

O instalador desktop precisa levar junto tudo o que os cores consomem fora do
executável. Nenhuma ROM, BIOS proprietária ou fonte externa é distribuída.

| Dependência | Consumida por | Como entra no instalador |
| --- | --- | --- |
| `MesenCore.dylib` / `.so` | todos os consoles Mesen (NES, SNES, GBA, GB, GBC, GG, SMS, WS, PCE) | `runtime/` → `bundle.resources` → `Contents/Resources/` |
| `libSDL2-2.0.0.dylib` / `libSDL2-2.0.so.0` | áudio do Mesen (`dlopen` de nome curto) | `runtime/` → `bundle.resources`; rpath `@loader_path`/`$ORIGIN` no Mesen |
| `n64wasm.js`, `n64wasm.wasm`, `assets.zip` | N64 (WebView, dentro do worker) | embutidos em `dist/wasm/n64/` no `frontendDist` **e** declarados em `bundle.resources` |
| BIOS GBA open-source | GBA no Mesen | `include_bytes!` no executável, gravada em `<home>/Firmware/gba_bios.bin` |
| BIOS GBA web | GBA no navegador | concatenada em `emmu_gba.js` pelo `tools/build-bridge.mjs` |
| Textos de licença | obrigação legal de todos os itens acima | `runtime/LICENSE-*.txt` e `THIRD-PARTY-NOTICES.txt` via `bundle.resources` |
| Cores WASM (10) | todos os consoles no navegador e N64 no desktop | embedados no executável como `frontendDist`; validados pelo `postbuild` |

## SDL2

O MesenCE **não linka** com SDL2. `Sdl/include/Sdl2.h` carrega a biblioteca em
runtime com `dlopen()` e apenas 6 funções de áudio (`SDL_InitSubSystem`,
`SDL_OpenAudioDevice`, `SDL_CloseAudioDevice`, `SDL_GetNumAudioDevices`,
`SDL_GetAudioDeviceName`, `SDL_PauseAudioDevice`). Isso foi confirmado com
`otool -L libMesenCore.dylib`: nenhuma entrada SDL2.

Para distribuir o Emmu sem exigir `brew install sdl2` ou `apt install libsdl2`,
basta levar a biblioteca junto e fazer o `dlopen` de nome curto encontrá-la.

### Versão

SDL2 **2.32.10** (tag `release-2.32.10` do
[libsdl-org/SDL](https://github.com/libsdl-org/SDL)), licença zlib. Foi
escolhido o SDL2 clássico em vez do `sdl2-compat` do Homebrew porque este é um
shim sobre SDL3 e puxaria uma segunda biblioteca. O submodule fica em
`vendor/SDL`, fixado nessa tag.

## Como gerar

```sh
git submodule update --init --recursive
# Prepara runtime Mesen/SDL, assets N64 e avisos de licença:
bash platforms/desktop/scripts/prepare-bundle.sh
# Equivalente ao passo de runtime do instalador:
./crates/emmu-mesen-sys/scripts/build-sdl2.sh
```

O `build-sdl2.sh`:

1. compila o SDL2 de `vendor/SDL` **somente com áudio** (desliga vídeo, render,
   joystick, X11/Wayland, PulseAudio etc. — o resultado é ~1,9 MB);
2. no macOS gera um binário universal (arm64 + x86_64) e ajusta o install name
   para `@rpath/libSDL2-2.0.0.dylib`;
3. copia `MesenCore` e `libSDL2` para `crates/emmu-mesen-sys/runtime/` e deixa
   em `runtime-link/` apenas o symlink usado pelo `-lMesenCore` do linker
   (separado porque dois nomes para o mesmo dylib no mesmo diretório fazem o
   dyld carregar o núcleo duas vezes e travam o `Emulator::Stop`);
4. ajusta o **install name** do `MesenCore` para `@rpath/MesenCore.dylib`
   (macOS). Sem isso o dyld ignora o `LC_RPATH` do executável e o app instalado
   fecha no launch com `Library not loaded: MesenCore.dylib`; o
   `prepare-bundle.sh` corrige automaticamente runtimes gerados por versões
   antigas do script;
5. adiciona `@loader_path` (macOS) / `$ORIGIN` (Linux, via `patchelf`) ao
   `MesenCore` — é isso que faz `dlopen("libSDL2-2.0.0.dylib")` resolver a
   cópia empacotada;
6. copia `LICENSE-SDL2.txt`.

O `prepare-bundle.sh` chama o `build-sdl2.sh` quando o runtime está incompleto e
ainda:

- copia `n64wasm.js`, `n64wasm.wasm` e `assets.zip` de
  `crates/emmu-n64/vendor/n64wasm/` para `platforms/web/ui/public/wasm/n64/`;
- copia os textos `LICENSE-N64WASM.txt`, `LICENSE-WEB-GBA.txt`,
  `LICENSE-GBA-BIOS.txt`, `LICENSE-GPL-3.0.txt`, `LICENSE-MESEN.txt` e
  `THIRD-PARTY-NOTICES.txt` para `runtime/`;
- falha com mensagem clara se faltar `MesenCore`, SDL2 ou algum asset do N64.

## Como entra no instalador

`platforms/desktop/tauri.conf.json` declara em `bundle.resources`:

```json
{
  "../../crates/emmu-mesen-sys/runtime/*": "",
  "../../crates/emmu-n64/vendor/n64wasm/n64wasm.js": "wasm/n64/n64wasm.js",
  "../../crates/emmu-n64/vendor/n64wasm/n64wasm.wasm": "wasm/n64/n64wasm.wasm",
  "../../crates/emmu-n64/vendor/n64wasm/assets.zip": "wasm/n64/assets.zip"
}
```

O destino precisa ser o **arquivo completo**, não o diretório `wasm/n64/`: o
Tauri interpreta essa última forma como "renomear o arquivo para `n64`" e o
bundle termina com um único arquivo chamado `wasm/n64`, sem os três assets.
Se isso aconteceu em um build antigo, remova os resíduos antes de reconstruir:

```sh
rm -rf target/release/wasm/n64 target/release/bundle/macos/Emmu.app \
       target/release/bundle/dmg/Emmu_0.2.0_aarch64.dmg
```

No macOS os arquivos vão para `$RESOURCE` (`Contents/Resources/`); `runtime-link/`
não entra no instalador. O `platforms/desktop/build.rs` adiciona o rpath
correspondente ao binário (`@executable_path/../Resources` no macOS,
`$ORIGIN/../lib` no Linux), e o `build.rs` deste crate passa a linkar a cópia
empacotada quando ela existe. Se `runtime/` não existir, o build volta ao
comportamento antigo (`MESEN_LIB_DIR` ou `/tmp/MesenCE/...`) para não quebrar o
desenvolvimento.

O frontend (incluindo `dist/wasm/n64/`) é embedado no executável porque o crate
desktop habilita `custom-protocol`. O script `postbuild` da UI confirma que os
dez módulos `_bg.wasm` e os três assets do N64 estão em `dist/`; sem isso o
instalador seria gerado com um core ausente.

## Verificação

Depois de `npm run desktop:build`, confira no `.app`:

```sh
APP=target/release/bundle/macos/Emmu.app/Contents/Resources
ls "$APP"                 # MesenCore, SDL2, LICENSE-*, THIRD-PARTY-NOTICES
ls "$APP/wasm/n64"        # n64wasm.js, n64wasm.wasm, assets.zip
otool -L "$APP/../MacOS/desktop" | grep MesenCore   # @rpath/MesenCore.dylib
```

Para desenvolvimento (`cargo tauri dev`) nada muda: o Mesen e o SDL2 do sistema
continuam sendo usados se `runtime/` não tiver sido gerado.
