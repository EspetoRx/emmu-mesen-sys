/** emmu/crates/emmu-mesen-sys/src/lib.rs */
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::atomic::{AtomicBool, Ordering};

pub const NOTIFICATION_PPU_FRAME_DONE: c_int = 7;

pub type NotificationListenerCallback = Option<extern "C" fn(c_int, *mut c_void)>;

static MESEN_SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub fn mark_shutting_down() {
    MESEN_SHUTTING_DOWN.store(true, Ordering::SeqCst);
}

pub fn is_shutting_down() -> bool {
    MESEN_SHUTTING_DOWN.load(Ordering::SeqCst)
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInfo {
    pub width: u32,
    pub height: u32,
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
    pub fn GetBaseScreenSize() -> FrameInfo;
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
    eprintln!("[emmu-mesen-sys] Port1/Port2 = NesController; ChannelVolumes = 100");
}

/// Configura o áudio do Mesen para o dispositivo padrão do SDL.
///
/// `AudioConfig::AudioDevice` é um `const char*` que nasce `nullptr` quando não
/// há `settings.json`. `SdlSoundManager::ProcessEndOfFrame()` chama
/// `SetAudioDevice(cfg.AudioDevice)` a cada frame, e um ponteiro nulo causa
/// `strlen(nullptr)` (SIGSEGV) na thread de emulação.
pub fn configure_audio() {
    set_audio_volume(100);
    eprintln!("[emmu-mesen-sys] AudioConfig ajustado para dispositivo padrão");
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
    eprintln!("[emmu-mesen-sys] SetInputOverrides index={index} state={state:?}");
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
        let (ok, ver) = ping();
        let date = build_date();
        println!("TestDll={ok}, GetMesenVersion={ver}, GetMesenBuildDate={date:?}");
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
            _ => {
                println!("Pulando test_mesen_load_rom: defina MESEN_TEST_ROM=/Users/lucas_carvalho/Documentos 2/emmu-nes-roms/roms/Adventure Island (USA).nes");
                return;
            }
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
        println!("ROM={rom_path}, loaded={loaded}, is_running={running_after_load}");

        if running_after_load {
            stop_emu();
        }

        unsafe {
            Release();
        }

        assert!(loaded, "LoadRom retornou false para {rom_path}");
    }
}
