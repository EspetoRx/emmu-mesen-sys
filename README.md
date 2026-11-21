# emmu-mesen-sys

Ligação FFI entre o Emmu desktop e o núcleo nativo **MesenCE/Mesen 2**. Este crate não contém o Mesen: ele declara a ABI C++ em Rust, fornece wrappers seguros, configura controladores/áudio, instala a BIOS GBA e prepara o runtime com SDL2 para distribuição.

> O Mesen é GPLv3 (`Copyright (C) 2014-2025 Sour`). O texto da licença não é versionado neste submódulo; obtenha-o do projeto upstream ao distribuir o binário.

## O que faz

- Declara estruturas de configuração com tamanhos e offsets exatos da ABI do MesenCE:
  - `NesConfigAbi` (14.184 bytes, align 4)
  - `SnesConfigAbi` (10.032 bytes, align 8)
  - `GbaConfigAbi` (1.048 bytes, align 4)
  - `GameboyConfigAbi` (2.084 bytes, align 4)
  - `PceConfigAbi` (8.084 bytes, align 4)
  - `SmsConfigAbi` (2.080 bytes, align 4)
  - `AudioConfigAbi` (240 bytes, align 8)
  - `SoftwareRendererSurfaceAbi` / `SoftwareRendererFrameAbi`
- Declara as funções `extern "C"` da DLL: `TestDll`, `GetMesenVersion`, `GetMesenBuildDate`, `InitDll`, `InitializeEmu`, `LoadRom`, `SaveStateFile`, `LoadStateFile`, `ExecuteShortcut`, `SetInputOverrides`, `RegisterNotificationCallback`, entre outras.
- Expõe wrappers Rust: `ping`, `build_date`, `install_gba_bios`, `load_rom`, `is_running`, `stop_emu`, `initialize_debugger`, `connect_nes_controllers`, `connect_snes_controllers`, `connect_gba_controllers`, `connect_gameboy_controllers`, `connect_gameboy_dmg_controllers`, `connect_sms_controllers`, `connect_pce_controllers`, `reset`, `configure_audio`, `set_audio_volume`, `save_state_to_file`, `load_state_from_file`, `set_input_overrides`, `release`, `mark_shutting_down` e `is_shutting_down`.
- Registra notificações `NOTIFICATION_PPU_FRAME_DONE = 7` e `NOTIFICATION_REFRESH_SOFTWARE_RENDERER = 22`.
- Possui um guard atômico de shutdown para impedir chamadas FFI depois de `Release()`; `release()` é idempotente.

## Configurações por console

| Console | Função de conexão | Detalhes |
| --- | --- | --- |
| NES | `connect_nes_controllers` | `ControllerType::NesController = 10`, volumes em offsets calculados por `offsetof` |
| SNES | `connect_snes_controllers` | `ControllerType::SnesController = 1`, crop/GSU/BS-X em offsets fixos |
| GBA | `connect_gba_controllers` | `ControllerType = 50`, `SkipBootScreen = 0` porque não há SWI HLE |
| GB/GBC | `connect_gameboy_controllers` / `connect_gameboy_dmg_controllers` | `ControllerType = 42`, `AutoFavorGbc = 1`, `Gameboy = 4` |
| PCE | `connect_pce_controllers` | `ControllerType = 44`, `PceConsoleType::TurboGrafx = 3`, paleta padrão em `PCE_PALETTE_OFFSET = 6036` |
| SMS/GG | `connect_sms_controllers` | `ControllerType = 47`, `GameGearOverscan = 48` nos 4 lados (frame 160x144), volumes e LCD/blend reaplicados |

`src/pce_palette.rs` contém `PCE_DEFAULT_PALETTE: [u32; 512]`, copiada de `MesenCE/UI/Config/PcEngineConfig.cs`. Sem ela, o filtro de vídeo PCE do Mesen renderiza tudo preto.

## Áudio e SDL2

O MesenCE **não linka** SDL2. `Sdl/include/Sdl2.h` carrega a biblioteca em runtime com `dlopen()` e usa apenas seis funções: `SDL_InitSubSystem`, `SDL_OpenAudioDevice`, `SDL_CloseAudioDevice`, `SDL_GetNumAudioDevices`, `SDL_GetAudioDeviceName` e `SDL_PauseAudioDevice`.

Por isso o Emmu empacota SDL2 ao lado do núcleo e ajusta o rpath:

- SDL2 **2.32.10**, tag `release-2.32.10`, licença zlib, submodule em `vendor/SDL`.
- macOS: instalador com `@rpath`, binário universal `arm64` + `x86_64`.
- Linux: `$ORIGIN` via `patchelf`.
- Configuração de áudio padrão: dispositivo `""`, 48 kHz, latência 60 ms, volume ≤ 100. Um `AudioDevice` nulo quebraria `SdlSoundManager`.

Detalhes completos em [`BUNDLE.md`](BUNDLE.md).

## BIOS GBA

`resources/gba_bios.bin` é incorporado com `include_bytes!`. `install_gba_bios()` grava `<home>/Firmware/gba_bios.bin` quando ainda não existe um arquivo de mesmo tamanho.

- Fonte: `https://github.com/Cult-of-GBA/BIOS`, commit `61158c3`.
- Licença MIT: `Copyright 2020 - 2021 DenSinH and fleroviux`.
- Texto em `LICENSE-GBA-BIOS.txt`.

## Árvore de arquivos

```text
emmu-mesen-sys/
├── BUNDLE.md                 # documentação do runtime SDL2/Mesen
├── Cargo.toml                # sem license/dependencies declaradas
├── LICENSE-GBA-BIOS.txt      # MIT, Cult-of-GBA/BIOS
├── build.rs                  # -L e -lMesenCore; usa runtime/, runtime-link/ ou MESEN_LIB_DIR
├── .gitmodules               # vendor/SDL
├── .gitignore                # ignora runtime/ e runtime-link/
├── resources/
│   └── gba_bios.bin          # BIOS GBA open-source incorporada
├── runtime/                  # gerado; vai para o bundle Tauri
│   ├── MesenCore.dylib / .so
│   ├── libSDL2-2.0.0.dylib / libSDL2-2.0.so.0
│   └── LICENSE-SDL2.txt
├── runtime-link/             # gerado; apenas symlink para o linker
│   └── libMesenCore.dylib / libMesenCore.so
├── scripts/
│   └── build-sdl2.sh         # compila SDL2 e prepara runtime/
├── src/
│   ├── lib.rs                # ABI, externs, wrappers, testes
│   └── pce_palette.rs        # paleta padrão PCE (512 entradas)
└── vendor/
    └── SDL/                  # submodule @ release-2.32.10
        ├── LICENSE.txt
        ├── README.md
        └── configure, CMakeLists.txt, include/, src/, ...
```

## Build e runtime

```sh
git submodule update --init --recursive

# macOS
./crates/emmu-mesen-sys/scripts/build-sdl2.sh

# Linux x86_64
MESEN_TARGET=x86_64-unknown-linux-gnu \
MESEN_LIB_DIR=/caminho/do/MesenCE \
./crates/emmu-mesen-sys/scripts/build-sdl2.sh
```

`build.rs` procura primeiro `runtime/<core>`; se não existir, usa `MESEN_LIB_DIR` ou os diretórios padrão `/tmp/MesenCE/InteropDLL/obj.osx-arm64`, `obj.linux-arm64` e `obj.linux-x64`.

## Testes

```sh
cargo test -p emmu-mesen-sys

MESEN_TEST_ROM="/caminho/jogo.nes" cargo test -p emmu-mesen-sys test_mesen_load_rom -- --nocapture
MESEN_PCE_TEST_ROM="/caminho/jogo.pce" cargo test -p emmu-mesen-sys test_mesen_load_pce_rom -- --nocapture
MESEN_GG_TEST_ROM="/caminho/jogo.gg" cargo test -p emmu-mesen-sys test_mesen_load_gg_rom -- --nocapture
```

Os testes exigem um `MesenCore` linkável.

## Limitações

- Windows ainda não é suportado: `build.rs` escolhe `.dylib` no macOS e `.so` em Linux; o script também só cobre esses targets.
- O firmware GBA é necessário porque o Mesen não faz SWI HLE nessa configuração.
- `noVideo` e `noAudio` parecem invertidos na implementação C++; o desktop passa `false` para ambos e lê frames via notificações.

## Licenças

| Componente | Licença | Detalhes |
| --- | --- | --- |
| `emmu-mesen-sys` | não declarada | sem `license` no `Cargo.toml` e sem `LICENSE` próprio |
| MesenCE / Mesen 2 | GPL-3.0 | `Copyright (C) 2014-2025 Sour`; binário externo |
| SDL2 2.32.10 | zlib | `Copyright (C) 1997-2025 Sam Lantinga <slouken@libsdl.org>` |
| BIOS GBA | MIT | `Copyright 2020 - 2021 DenSinH and fleroviux` |
| Paleta PCE | derivada do MesenCE | copiada de `PcEngineConfig.cs`, GPLv3 |

Ao distribuir o aplicativo, inclua os textos de licença do Mesen, SDL2 e BIOS, e cumpra a GPLv3 para o núcleo.
