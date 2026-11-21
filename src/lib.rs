/** emmu/crates/emmu-mesen-sys/src/lib.rs */
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub const NOTIFICATION_PPU_FRAME_DONE: c_int = 7;
/// `SoftwareRenderer::Render()` envia um `SoftwareRendererFrame` com buffer
/// ARGB pronto (o `PpuFrameDone` do SNES não passa o buffer como parâmetro).
pub const NOTIFICATION_REFRESH_SOFTWARE_RENDERER: c_int = 22;

pub type NotificationListenerCallback = Option<extern "C" fn(c_int, *mut c_void)>;

static MESEN_SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub fn mark_shutting_down() {
    MESEN_SHUTTING_DOWN.store(true, Ordering::SeqCst);
}

pub fn is_shutting_down() -> bool {
    MESEN_SHUTTING_DOWN.load(Ordering::SeqCst)
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugControllerState {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub l: bool,
    pub r: bool,
    pub u: bool,
    pub d: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub start: bool,
}

/// Tamanho de `NesConfig` na ABI do MesenCE (ver Core/Shared/SettingTypes.h).
pub const NES_CONFIG_SIZE: usize = 14184;

/// Cópia opaca de `NesConfig`. O alinhamento de 4 é necessário para casar
/// com a ABI da estrutura C++ ao passá-la por valor pelo FFI.
#[repr(C, align(4))]
pub struct NesConfigAbi {
    pub bytes: [u8; NES_CONFIG_SIZE],
}

/// `offsetof(NesConfig, Port1.Type)` / `Port2.Type`.
pub const NES_PORT1_TYPE_OFFSET: usize = 988;
pub const NES_PORT2_TYPE_OFFSET: usize = 1980;
/// `ControllerType::NesController` na enum do MesenCE.
pub const CONTROLLER_TYPE_NES_CONTROLLER: u32 = 10;

/// Offsets calculados com `offsetof(NesConfig, ...)` contra os headers do MesenCE.
/// Sem `settings.json`, `NesConfig::ChannelVolumes` nasce todo zerado e o
/// `NesSoundMixer` calcula `_volumes[i] = 0 / 100.0` — silêncio total.
pub const NES_CHANNEL_VOLUMES_OFFSET: usize = 14072;
pub const NES_CHANNEL_COUNT: usize = 11;
pub const NES_EPSM_VOLUME_OFFSET: usize = 14116;

/// Tamanho de `SnesConfig` na ABI do MesenCE (ver Core/Shared/SettingTypes.h).
pub const SNES_CONFIG_SIZE: usize = 10032;

/// Cópia opaca de `SnesConfig`. O alinhamento de 8 casa com o `int64_t`
/// `BsxCustomDate` no fim da estrutura C++.
#[repr(C, align(8))]
pub struct SnesConfigAbi {
    pub bytes: [u8; SNES_CONFIG_SIZE],
}

/// `offsetof(SnesConfig, Port1.Type)` / `Port2.Type` — as mesmas de NES,
/// porque `ControllerConfig` tem o mesmo layout.
pub const SNES_PORT1_TYPE_OFFSET: usize = 988;
pub const SNES_PORT2_TYPE_OFFSET: usize = 1980;
/// `ControllerType::SnesController` na enum do MesenCE.
pub const CONTROLLER_TYPE_SNES_CONTROLLER: u32 = 1;

/// `SnesConfig::ChannelVolumes[8]` — diferente do NES, o default é 100, mas
/// como reconstruímos a estrutura zerada precisamos escrever 100 em cada um.
pub const SNES_CHANNEL_VOLUMES_OFFSET: usize = 9968;
pub const SNES_CHANNEL_COUNT: usize = 8;
/// `SnesConfig::Overscan` (Left, Right, Top, Bottom) — aplica o mesmo crop
/// que a UI do Mesen usa, deixando o frame 256x224 igual ao core WASM web.
pub const SNES_OVERSCAN_OFFSET: usize = 9948;
/// `SnesConfig::GsuClockSpeed` e `BsxCustomDate` — os únicos campos cujo
/// valor padrão C++ não é zero (todos os enums começam em 0 = Auto/None/Weave).
pub const SNES_GSU_CLOCK_SPEED_OFFSET: usize = 10020;
pub const SNES_BSX_CUSTOM_DATE_OFFSET: usize = 10024;

/// Tamanho de `GbaConfig` na ABI do MesenCE (confirmado com `offsetof`).
pub const GBA_CONFIG_SIZE: usize = 1048;

/// Cópia opaca de `GbaConfig`. Alinhamento 4, igual ao da estrutura C++.
#[repr(C, align(4))]
pub struct GbaConfigAbi {
    pub bytes: [u8; GBA_CONFIG_SIZE],
}

/// `offsetof(GbaConfig, Controller.Type)` e
/// `ControllerType::GbaController` na enum do MesenCE.
pub const GBA_CONTROLLER_TYPE_OFFSET: usize = 988;
pub const CONTROLLER_TYPE_GBA_CONTROLLER: u32 = 50;

/// `GbaConfig::SkipBootScreen`. Mantemos `false`: o Mesen não tem HLE de SWI
/// e os jogos dependem das rotinas do BIOS; pular o boot trava a CPU em
/// `DISPCNT=0x80` (tela branca) mesmo com a ROM carregada.
pub const GBA_SKIP_BOOT_SCREEN_OFFSET: usize = 992;
/// Defaults C++ de `GbaConfig` que precisam ser reaplicados porque montamos a
/// estrutura zerada (`SnesConfig` já faz o mesmo para overscan).
pub const GBA_BLEND_FRAMES_OFFSET: usize = 994;
pub const GBA_ADJUST_COLORS_OFFSET: usize = 995;
/// `GbaConfig::ChannelAVol` (6 volumes u32 em sequência).
pub const GBA_CHANNEL_VOLUMES_OFFSET: usize = 1024;
pub const GBA_CHANNEL_COUNT: usize = 6;

/// Tamanho de `GameboyConfig` na ABI do MesenCE (confirmado com `offsetof`).
pub const GAMEBOY_CONFIG_SIZE: usize = 2084;

/// Cópia opaca de `GameboyConfig`. Alinhamento 4, igual ao da estrutura C++.
#[repr(C, align(4))]
pub struct GameboyConfigAbi {
    pub bytes: [u8; GAMEBOY_CONFIG_SIZE],
}

/// `offsetof(GameboyConfig, Controller.Type)` / `LinkedController.Type`.
pub const GAMEBOY_PORT1_TYPE_OFFSET: usize = 988;
pub const GAMEBOY_PORT2_TYPE_OFFSET: usize = 1980;
/// `ControllerType::GameboyController` na enum do MesenCE.
pub const CONTROLLER_TYPE_GAMEBOY_CONTROLLER: u32 = 42;

/// Defaults C++ de `GameboyConfig` que precisam ser reaplicados porque a ABI
/// não exporta `GetGameboyConfig` e montamos a estrutura zerada.
pub const GAMEBOY_MODEL_OFFSET: usize = 1984;
pub const GAMEBOY_USE_SGB2_OFFSET: usize = 1988;
pub const GAMEBOY_BLEND_FRAMES_OFFSET: usize = 2000;
pub const GAMEBOY_ADJUST_COLORS_OFFSET: usize = 2001;
pub const GAMEBOY_RAM_POWER_ON_STATE_OFFSET: usize = 2008;
pub const GAMEBOY_BG_COLORS_OFFSET: usize = 2020;
pub const GAMEBOY_OBJ0_COLORS_OFFSET: usize = 2036;
pub const GAMEBOY_OBJ1_COLORS_OFFSET: usize = 2052;
pub const GAMEBOY_CHANNEL_VOLUMES_OFFSET: usize = 2068;
pub const GAMEBOY_CHANNEL_COUNT: usize = 4;
/// `GameboyModel::AutoFavorGbc`: executa carts CGB como CGB e carts DMG no
/// modo de compatibilidade colorido pelo boot ROM do CGB.
pub const GAMEBOY_MODEL_AUTO_FAVOR_GBC: u32 = 1;
/// `GameboyModel::Gameboy`: força o DMG clássico (sem colorização CGB).
pub const GAMEBOY_MODEL_GAMEBOY: u32 = 4;

/// Tamanho de `PcEngineConfig` na ABI do MesenCE (confirmado com `offsetof`).
pub const PCE_CONFIG_SIZE: usize = 8084;

/// Cópia opaca de `PcEngineConfig`. Alinhamento 4, igual ao da estrutura C++.
#[repr(C, align(4))]
pub struct PceConfigAbi {
    pub bytes: [u8; PCE_CONFIG_SIZE],
}

/// `offsetof(PcEngineConfig, Port1.Type)`.
pub const PCE_PORT1_TYPE_OFFSET: usize = 988;
/// `ControllerType::PceController` na enum do MesenCE.
pub const CONTROLLER_TYPE_PCE_CONTROLLER: u32 = 44;
/// `PceConsoleType::TurboGrafx` (evita o travamento de region locking de jogos
/// norte-americanos quando `Auto` é usado, ver `PceMemoryManager.cpp`).
pub const PCE_CONSOLE_TYPE_TURBOGRAFX: u32 = 3;
/// Defaults C++ de `PcEngineConfig` que precisam ser reaplicados porque a ABI
/// não exporta `GetPcEngineConfig` e montamos a estrutura zerada.
pub const PCE_CONSOLE_TYPE_OFFSET: usize = 5956;
pub const PCE_ADPCM_CLOCK_SPEED_OFFSET: usize = 5976;
pub const PCE_CHANNEL_VOLUMES_OFFSET: usize = 5980;
pub const PCE_CHANNEL_COUNT: usize = 6;
pub const PCE_CD_AUDIO_VOLUME_OFFSET: usize = 6004;
pub const PCE_ADPCM_VOLUME_OFFSET: usize = 6008;
pub const PCE_USE_HUC6280A_AUDIO_OFFSET: usize = 6012;

/// Parâmetro de `ExecuteShortcut` — `EmulatorShortcut::Reset` na ABI do MesenCE.
#[repr(C)]
pub struct ExecuteShortcutParams {
    pub shortcut: u32,
    pub param: u32,
    pub param_ptr: *mut c_void,
}
pub const SHORTCUT_RESET: u32 = 30;

/// Structs enviadas por `RefreshSoftwareRenderer` (SoftwareRenderer.cpp).
#[repr(C)]
pub struct SoftwareRendererSurfaceAbi {
    pub buffer: *const u32,
    pub width: u32,
    pub height: u32,
    pub is_dirty: bool,
}

#[repr(C)]
pub struct SoftwareRendererFrameAbi {
    pub frame: SoftwareRendererSurfaceAbi,
    pub emu_hud: SoftwareRendererSurfaceAbi,
    pub script_hud: SoftwareRendererSurfaceAbi,
}

/// Tamanho/alinhamento de `AudioConfig` na ABI do MesenCE.
pub const AUDIO_CONFIG_SIZE: usize = 240;
const AUDIO_DEVICE_OFFSET: usize = 8;
const AUDIO_ENABLE_OFFSET: usize = 16;
const AUDIO_MASTER_VOLUME_OFFSET: usize = 20;
const AUDIO_SAMPLE_RATE_OFFSET: usize = 24;
const AUDIO_LATENCY_OFFSET: usize = 28;

#[repr(C, align(8))]
pub struct AudioConfigAbi {
    bytes: [u8; AUDIO_CONFIG_SIZE],
}

/// Nome de dispositivo vazio: diz ao SDL para usar o dispositivo padrão.
/// Precisa ser `static` porque o Mesen guarda o ponteiro em `AudioConfig`.
static DEFAULT_AUDIO_DEVICE: [u8; 1] = [0];

#[link(name = "MesenCore")]
extern "C" {
    pub fn TestDll() -> bool;
    pub fn GetMesenVersion() -> u32;
    pub fn GetMesenBuildDate() -> *const c_char;
    pub fn GetNesConfig() -> NesConfigAbi;
    pub fn SetNesConfig(config: NesConfigAbi);
    pub fn SetSnesConfig(config: SnesConfigAbi);
    pub fn SetGbaConfig(config: GbaConfigAbi);
    pub fn SetGameboyConfig(config: GameboyConfigAbi);
    pub fn SetPcEngineConfig(config: PceConfigAbi);
    pub fn ExecuteShortcut(params: ExecuteShortcutParams);
    pub fn SetAudioConfig(config: AudioConfigAbi);
    pub fn SaveStateFile(filepath: *mut c_char);
    pub fn LoadStateFile(filepath: *mut c_char);
    pub fn InitDll();
    pub fn InitializeEmu(
        homeFolder: *const c_char,
        windowHandle: *mut c_void,
        viewerHandle: *mut c_void,
        softwareRenderer: bool,
        noAudio: bool,
        noVideo: bool,
        noInput: bool,
    );
    pub fn Release();
    pub fn LoadRom(filename: *mut c_char, patchFile: *mut c_char) -> bool;
    pub fn IsRunning() -> bool;
    pub fn Stop();
    pub fn RegisterNotificationCallback(callback: NotificationListenerCallback) -> *mut c_void;
    pub fn UnregisterNotificationCallback(listener: *mut c_void);
    pub fn InitializeDebugger();
    pub fn ReleaseDebugger();
    pub fn SetInputOverrides(index: u32, state: DebugControllerState);
}

pub fn ping() -> (bool, u32) {
    unsafe { (TestDll(), GetMesenVersion()) }
}

pub fn build_date() -> Option<String> {
    unsafe {
        let ptr = GetMesenBuildDate();
        if ptr.is_null() {
            return None;
        }
        Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
    }
}

/// BIOS GBA open-source (MIT) usada quando o usuário não forneceu uma.
///
/// O Mesen não emula as chamadas SWI em alto nível: sem `gba_bios.bin` os
/// jogos executam zeros no vetor de software interrupt e nunca limpam
/// `DISPCNT.ForcedBlank`, ficando com a tela branca.
const GBA_BIOS: &[u8] = include_bytes!("../resources/gba_bios.bin");

/// Instala a BIOS em `<home>/Firmware/gba_bios.bin` antes de `LoadRom`.
///
/// Um arquivo existente com o tamanho correto é preservado, para não
/// sobrescrever uma BIOS oficial fornecida pelo usuário.
pub fn install_gba_bios(home_folder: &str) -> Result<(), std::io::Error> {
    let path = Path::new(home_folder).join("Firmware").join("gba_bios.bin");
    if let Ok(metadata) = std::fs::metadata(&path) {
        if metadata.len() == GBA_BIOS.len() as u64 {
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, GBA_BIOS)
}

pub fn load_rom(rom_path: &str, patch_path: Option<&str>) -> Result<bool, std::ffi::NulError> {
    if is_shutting_down() {
        return Ok(false);
    }

    let rom_c = CString::new(rom_path)?;
    let rom_ptr = rom_c.into_raw();

    let patch_ptr = match patch_path {
        Some(p) => CString::new(p)?.into_raw(),
        None => std::ptr::null_mut(),
    };

    let loaded = unsafe { LoadRom(rom_ptr, patch_ptr) };

    unsafe {
        drop(CString::from_raw(rom_ptr));
        if !patch_ptr.is_null() {
            drop(CString::from_raw(patch_ptr));
        }
    }

    Ok(loaded)
}

pub fn is_running() -> bool {
    if is_shutting_down() {
        return false;
    }
    unsafe { IsRunning() }
}

pub fn stop_emu() {
    if is_shutting_down() {
        return;
    }
    unsafe { Stop() }
}

pub fn initialize_debugger() {
    if is_shutting_down() {
        return;
    }
    unsafe {
        InitializeDebugger();
    }
}

pub fn release_debugger() {
    if is_shutting_down() {
        return;
    }
    unsafe {
        ReleaseDebugger();
    }
}

/// Configura controles e volumes do NES para uma instância headless.
///
/// Sem `settings.json`, `Port1.Type`/`Port2.Type` ficam como
/// `ControllerType::None` (o que impede `SetInputOverrides` de funcionar) e
/// `ChannelVolumes[11]` fica todo zerado (o que zera todo o áudio no
/// `NesSoundMixer`). Corrigimos os dois aqui.
pub fn connect_nes_controllers() {
    if is_shutting_down() {
        return;
    }
    let mut config = unsafe { GetNesConfig() };

    // Sempre conecta as duas portas. O "desconectar" do P2 é feito no Player
    // (teclas P2 ignoradas + soltar overrides), porque o MesenCE não reconstrói
    // os devices de controle em runtime quando NesConfig muda (o noisy
    // `RequestConfigChange` só é tratado pela UI C# que não existe aqui).
    let controller = CONTROLLER_TYPE_NES_CONTROLLER.to_le_bytes();
    config.bytes[NES_PORT1_TYPE_OFFSET..NES_PORT1_TYPE_OFFSET + 4].copy_from_slice(&controller);
    config.bytes[NES_PORT2_TYPE_OFFSET..NES_PORT2_TYPE_OFFSET + 4].copy_from_slice(&controller);

    let volume = 100u32.to_le_bytes();
    for i in 0..NES_CHANNEL_COUNT {
        let offset = NES_CHANNEL_VOLUMES_OFFSET + i * 4;
        config.bytes[offset..offset + 4].copy_from_slice(&volume);
    }
    config.bytes[NES_EPSM_VOLUME_OFFSET..NES_EPSM_VOLUME_OFFSET + 4].copy_from_slice(&volume);

    unsafe { SetNesConfig(config) };
}

/// Configura controles, volumes e overscan do SNES para uma instância headless.
///
/// Chamado antes de `LoadRom`, porque o MesenCE cria os control devices uma
/// única vez durante o carregamento e não os reconstrói quando a config muda
/// em runtime (o `RequestConfigChange` só é tratado pela UI C#).
pub fn connect_snes_controllers() {
    if is_shutting_down() {
        return;
    }
    // A ABI do MesenCE não exporta GetSnesConfig, então montamos a estrutura
    // do zero. Todos os enums de SnesConfig têm valor 0 = padrão do C++
    // (ConsoleRegion::Auto, RamState::Random, DspInterpolationType::Gauss,
    // SnesColorCorrectionMode::None, SnesDeinterlaceMode::Weave, ...).
    let mut config = SnesConfigAbi {
        bytes: [0; SNES_CONFIG_SIZE],
    };

    let controller = CONTROLLER_TYPE_SNES_CONTROLLER.to_le_bytes();
    config.bytes[SNES_PORT1_TYPE_OFFSET..SNES_PORT1_TYPE_OFFSET + 4].copy_from_slice(&controller);
    config.bytes[SNES_PORT2_TYPE_OFFSET..SNES_PORT2_TYPE_OFFSET + 4].copy_from_slice(&controller);

    let volume = 100u32.to_le_bytes();
    for i in 0..SNES_CHANNEL_COUNT {
        let offset = SNES_CHANNEL_VOLUMES_OFFSET + i * 4;
        config.bytes[offset..offset + 4].copy_from_slice(&volume);
    }

    // Left / Right / Top / Bottom = 0 / 0 / 7 / 8 (padrão da UI do Mesen),
    // deixando o frame 256x224 igual ao core WASM usado na web.
    for (i, value) in [0u32, 0, 7, 8].iter().enumerate() {
        let offset = SNES_OVERSCAN_OFFSET + i * 4;
        config.bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    config.bytes[SNES_GSU_CLOCK_SPEED_OFFSET..SNES_GSU_CLOCK_SPEED_OFFSET + 4]
        .copy_from_slice(&100u32.to_le_bytes());
    config.bytes[SNES_BSX_CUSTOM_DATE_OFFSET..SNES_BSX_CUSTOM_DATE_OFFSET + 8]
        .copy_from_slice(&(-1i64).to_le_bytes());

    unsafe { SetSnesConfig(config) };
}

/// Configura controle, boot com BIOS e volumes do GBA para uma instância headless.
///
/// Chamado antes de `LoadRom`, porque o MesenCE cria os control devices apenas
/// uma vez durante o carregamento da ROM.
pub fn connect_gba_controllers() {
    if is_shutting_down() {
        return;
    }
    let mut config = GbaConfigAbi {
        bytes: [0; GBA_CONFIG_SIZE],
    };

    let controller = CONTROLLER_TYPE_GBA_CONTROLLER.to_le_bytes();
    config.bytes[GBA_CONTROLLER_TYPE_OFFSET..GBA_CONTROLLER_TYPE_OFFSET + 4]
        .copy_from_slice(&controller);

    // Não pular o boot: o Mesen não tem HLE de SWI e o boot ROM real (instalado
    // por `install_gba_bios`) é necessário para as rotinas do BIOS usadas pelos
    // jogos. Com `SkipBootScreen=true` a tela fica branca (DISPCNT=0x80).
    config.bytes[GBA_SKIP_BOOT_SCREEN_OFFSET] = 0;
    config.bytes[GBA_BLEND_FRAMES_OFFSET] = 1;
    config.bytes[GBA_ADJUST_COLORS_OFFSET] = 1;

    let volume = 100u32.to_le_bytes();
    for i in 0..GBA_CHANNEL_COUNT {
        let offset = GBA_CHANNEL_VOLUMES_OFFSET + i * 4;
        config.bytes[offset..offset + 4].copy_from_slice(&volume);
    }

    unsafe { SetGbaConfig(config) };
}

/// Configura controle, modelo e volumes de Game Boy para uma instância headless.
/// Chamado antes de `LoadRom`, porque o MesenCE cria os control devices apenas
/// uma vez durante o carregamento da ROM.
pub fn connect_gameboy_controllers_with_model(model: u32) {
    if is_shutting_down() {
        return;
    }
    // A ABI não exporta GetGameboyConfig, então reconstruímos a estrutura.
    // Campos deixados em zero já têm o valor padrão do C++ (link cable desligado,
    // RamState::Random, sem overlays, overclock 0, etc.).
    let mut config = GameboyConfigAbi {
        bytes: [0; GAMEBOY_CONFIG_SIZE],
    };

    let controller = CONTROLLER_TYPE_GAMEBOY_CONTROLLER.to_le_bytes();
    config.bytes[GAMEBOY_PORT1_TYPE_OFFSET..GAMEBOY_PORT1_TYPE_OFFSET + 4]
        .copy_from_slice(&controller);
    config.bytes[GAMEBOY_PORT2_TYPE_OFFSET..GAMEBOY_PORT2_TYPE_OFFSET + 4]
        .copy_from_slice(&controller);

    config.bytes[GAMEBOY_MODEL_OFFSET..GAMEBOY_MODEL_OFFSET + 4]
        .copy_from_slice(&model.to_le_bytes());
    config.bytes[GAMEBOY_USE_SGB2_OFFSET] = 1;
    config.bytes[GAMEBOY_BLEND_FRAMES_OFFSET] = 1;
    config.bytes[GAMEBOY_ADJUST_COLORS_OFFSET] = 1;
    // RamState::Random = 0 (mantido em zero).

    // Paletas monocromáticas padrão da UI do Mesen.
    let colors = [0xFFFFFFu32, 0xB0B0B0, 0x686868, 0x000000];
    for (offset, palette) in [
        (GAMEBOY_BG_COLORS_OFFSET, colors),
        (GAMEBOY_OBJ0_COLORS_OFFSET, colors),
        (GAMEBOY_OBJ1_COLORS_OFFSET, colors),
    ] {
        for (index, color) in palette.iter().enumerate() {
            let at = offset + index * 4;
            config.bytes[at..at + 4].copy_from_slice(&color.to_le_bytes());
        }
    }

    let volume = 100u32.to_le_bytes();
    for index in 0..GAMEBOY_CHANNEL_COUNT {
        let offset = GAMEBOY_CHANNEL_VOLUMES_OFFSET + index * 4;
        config.bytes[offset..offset + 4].copy_from_slice(&volume);
    }

    unsafe { SetGameboyConfig(config) };
}

/// Game Boy Color: carts CGB em modo CGB e carts DMG coloridos pelo boot ROM.
pub fn connect_gameboy_controllers() {
    connect_gameboy_controllers_with_model(GAMEBOY_MODEL_AUTO_FAVOR_GBC);
}

/// Game Boy clássico (DMG): força o modo monocromático antes de `LoadRom`.
pub fn connect_gameboy_dmg_controllers() {
    connect_gameboy_controllers_with_model(GAMEBOY_MODEL_GAMEBOY);
}

/// Configura controle, console e volumes de PC Engine para uma instância
/// headless. Chamado antes de `LoadRom`, porque o MesenCE cria os control
/// devices apenas uma vez durante o carregamento da ROM.
pub fn connect_pce_controllers() {
    if is_shutting_down() {
        return;
    }
    // A ABI não exporta GetPcEngineConfig, então reconstruímos a estrutura.
    // Campos deixados em zero já têm o valor padrão do C++ (sem CD-ROM,
    // RamState::Random, sem overlays, etc.).
    let mut config = PceConfigAbi {
        bytes: [0; PCE_CONFIG_SIZE],
    };

    let controller = CONTROLLER_TYPE_PCE_CONTROLLER.to_le_bytes();
    config.bytes[PCE_PORT1_TYPE_OFFSET..PCE_PORT1_TYPE_OFFSET + 4].copy_from_slice(&controller);
    config.bytes[PCE_ADPCM_CLOCK_SPEED_OFFSET..PCE_ADPCM_CLOCK_SPEED_OFFSET + 4]
        .copy_from_slice(&32100u32.to_le_bytes());

    // `PceConsoleType::TurboGrafx` força o bit de região norte-americana na
    // MEM 0x1960; com `Auto` alguns jogos (ex.: Aero Blasters) travam.
    config.bytes[PCE_CONSOLE_TYPE_OFFSET..PCE_CONSOLE_TYPE_OFFSET + 4]
        .copy_from_slice(&PCE_CONSOLE_TYPE_TURBOGRAFX.to_le_bytes());

    let volume = 100u32.to_le_bytes();
    for index in 0..PCE_CHANNEL_COUNT {
        let offset = PCE_CHANNEL_VOLUMES_OFFSET + index * 4;
        config.bytes[offset..offset + 4].copy_from_slice(&volume);
    }
    for offset in [PCE_CD_AUDIO_VOLUME_OFFSET, PCE_ADPCM_VOLUME_OFFSET] {
        config.bytes[offset..offset + 4].copy_from_slice(&volume);
    }
    config.bytes[PCE_USE_HUC6280A_AUDIO_OFFSET] = 1;

    unsafe { SetPcEngineConfig(config) };
}

/// Reinicia o jogo atual pelo atalho interno do Mesen (equivalente ao reset).
pub fn reset() {
    if is_shutting_down() {
        return;
    }
    unsafe {
        ExecuteShortcut(ExecuteShortcutParams {
            shortcut: SHORTCUT_RESET,
            param: 0,
            param_ptr: std::ptr::null_mut(),
        });
    }
}

/// Configura o áudio do Mesen para o dispositivo padrão do SDL.
///
/// `AudioConfig::AudioDevice` é um `const char*` que nasce `nullptr` quando não
/// há `settings.json`. `SdlSoundManager::ProcessEndOfFrame()` chama
/// `SetAudioDevice(cfg.AudioDevice)` a cada frame, e um ponteiro nulo causa
/// `strlen(nullptr)` (SIGSEGV) na thread de emulação.
pub fn configure_audio() {
    set_audio_volume(100);
}

fn audio_config(volume: u32) -> AudioConfigAbi {
    let mut config = AudioConfigAbi {
        bytes: [0; AUDIO_CONFIG_SIZE],
    };
    // AudioDevice = "" (dispositivo padrão).
    config.bytes[AUDIO_DEVICE_OFFSET..AUDIO_DEVICE_OFFSET + 8]
        .copy_from_slice(&(DEFAULT_AUDIO_DEVICE.as_ptr() as usize).to_le_bytes());
    config.bytes[AUDIO_ENABLE_OFFSET] = 1;
    config.bytes[AUDIO_MASTER_VOLUME_OFFSET..AUDIO_MASTER_VOLUME_OFFSET + 4]
        .copy_from_slice(&volume.min(100).to_le_bytes());
    config.bytes[AUDIO_SAMPLE_RATE_OFFSET..AUDIO_SAMPLE_RATE_OFFSET + 4]
        .copy_from_slice(&48000u32.to_le_bytes());
    config.bytes[AUDIO_LATENCY_OFFSET..AUDIO_LATENCY_OFFSET + 4]
        .copy_from_slice(&60u32.to_le_bytes());
    config
}

/// Ajusta o volume de saída do Mesen (0-100). Não há `GetAudioConfig` na ABI,
/// então reconstruímos a config com os mesmos valores de `configure_audio()`.
pub fn set_audio_volume(volume: u32) {
    if is_shutting_down() {
        return;
    }
    unsafe { SetAudioConfig(audio_config(volume)) };
}

pub fn save_state_to_file(path: &str) -> Result<(), std::ffi::NulError> {
    if is_shutting_down() {
        return Ok(());
    }
    let path = CString::new(path)?;
    unsafe { SaveStateFile(path.as_ptr() as *mut c_char) };
    Ok(())
}

pub fn load_state_from_file(path: &str) -> Result<(), std::ffi::NulError> {
    if is_shutting_down() {
        return Ok(());
    }
    let path = CString::new(path)?;
    unsafe { LoadStateFile(path.as_ptr() as *mut c_char) };
    Ok(())
}

pub fn set_input_overrides(index: u32, state: DebugControllerState) {
    if is_shutting_down() {
        return;
    }
    unsafe {
        SetInputOverrides(index, state);
    }
}

pub fn release() {
    if MESEN_SHUTTING_DOWN.swap(true, Ordering::SeqCst) {
        return;
    }
    unsafe {
        Release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesen_ping_and_build_date() {
        let (ok, _ver) = ping();
        let date = build_date();
        assert!(ok);
        assert!(date.is_some());
    }

    #[test]
    fn test_mesen_headless_lifecycle() {
        let home = CString::new("/tmp/mesen_home").unwrap();
        unsafe {
            InitializeEmu(
                home.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                false,
                true,
                true,
                true,
            );
            InitDll();
            Release();
        }
    }

    #[test]
    fn test_mesen_load_rom() {
        let rom_path = match std::env::var("MESEN_TEST_ROM") {
            Ok(p) if !p.is_empty() => p,
            _ => return,
        };

        let home = CString::new("/tmp/mesen_home").unwrap();
        unsafe {
            InitializeEmu(
                home.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                false,
                true,
                true,
                true,
            );
            InitDll();
        }

        let loaded = load_rom(&rom_path, None).expect("caminho com byte nulo");
        let running_after_load = is_running();

        if running_after_load {
            stop_emu();
        }

        unsafe {
            Release();
        }

        assert!(loaded, "LoadRom retornou false para {rom_path}");
    }

    #[test]
    fn test_mesen_load_pce_rom() {
        let rom_path = match std::env::var("MESEN_PCE_TEST_ROM") {
            Ok(p) if !p.is_empty() => p,
            _ => return,
        };

        let home = CString::new("/tmp/mesen_home").unwrap();
        unsafe {
            InitializeEmu(
                home.as_ptr(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                false,
                true,
                true,
                true,
            );
            InitDll();
        }

        connect_pce_controllers();
        eprintln!("PCE FFI: chamando LoadRom...");
        let loaded = load_rom(&rom_path, None).expect("caminho com byte nulo");
        eprintln!("PCE FFI: LoadRom={loaded}");
        let running_after_load = is_running();
        eprintln!("PCE FFI: IsRunning={running_after_load}");

        if running_after_load {
            stop_emu();
            eprintln!("PCE FFI: Stop concluído");
        }

        unsafe {
            Release();
        }
        eprintln!("PCE FFI: Release concluído");

        assert!(loaded, "LoadRom retornou false para {rom_path}");
        assert!(running_after_load, "PCE não iniciou após LoadRom");
    }
}
