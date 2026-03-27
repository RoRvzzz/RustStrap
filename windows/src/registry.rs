use anyhow::{anyhow, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegistryHive {
    CurrentUser,
    LocalMachine,
    ClassesRoot,
    Users,
    CurrentConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegistryValue {
    String(String),
    ExpandString(String),
    Dword(u32),
    Qword(u64),
    Binary(Vec<u8>),
    MultiString(Vec<String>),
}

pub trait RegistryBackend {
    fn set_value(
        &self,
        hive: RegistryHive,
        key_path: &str,
        value_name: &str,
        value: RegistryValue,
    ) -> Result<()>;

    fn get_value(
        &self,
        hive: RegistryHive,
        key_path: &str,
        value_name: &str,
    ) -> Result<Option<RegistryValue>>;

    fn delete_value(&self, hive: RegistryHive, key_path: &str, value_name: &str) -> Result<()>;

    fn delete_key(&self, hive: RegistryHive, key_path: &str) -> Result<()>;

    fn key_exists(&self, hive: RegistryHive, key_path: &str) -> Result<bool>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WindowsRegistryBackend;

#[cfg(windows)]
impl RegistryBackend for WindowsRegistryBackend {
    fn set_value(
        &self,
        hive: RegistryHive,
        key_path: &str,
        value_name: &str,
        value: RegistryValue,
    ) -> Result<()> {
        use winreg::enums::{
            RegType, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
            HKEY_USERS,
        };
        use winreg::{RegKey, RegValue};

        let root = RegKey::predef(match hive {
            RegistryHive::CurrentUser => HKEY_CURRENT_USER,
            RegistryHive::LocalMachine => HKEY_LOCAL_MACHINE,
            RegistryHive::ClassesRoot => HKEY_CLASSES_ROOT,
            RegistryHive::Users => HKEY_USERS,
            RegistryHive::CurrentConfig => HKEY_CURRENT_CONFIG,
        });

        let (key, _) = root.create_subkey(key_path)?;

        match value {
            RegistryValue::String(value) => key.set_value(value_name, &value)?,
            RegistryValue::ExpandString(value) => {
                key.set_raw_value(
                    value_name,
                    &RegValue {
                        bytes: utf16_bytes_with_double_nul(&[value]),
                        vtype: RegType::REG_EXPAND_SZ,
                    },
                )?;
            }
            RegistryValue::Dword(value) => key.set_value(value_name, &value)?,
            RegistryValue::Qword(value) => key.set_value(value_name, &value)?,
            RegistryValue::Binary(value) => key.set_raw_value(
                value_name,
                &RegValue {
                    bytes: value,
                    vtype: RegType::REG_BINARY,
                },
            )?,
            RegistryValue::MultiString(value) => key.set_raw_value(
                value_name,
                &RegValue {
                    bytes: utf16_bytes_with_double_nul(&value),
                    vtype: RegType::REG_MULTI_SZ,
                },
            )?,
        }

        Ok(())
    }

    fn get_value(
        &self,
        hive: RegistryHive,
        key_path: &str,
        value_name: &str,
    ) -> Result<Option<RegistryValue>> {
        use winreg::enums::{
            RegType, HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
            HKEY_USERS,
        };
        use winreg::RegKey;

        let root = RegKey::predef(match hive {
            RegistryHive::CurrentUser => HKEY_CURRENT_USER,
            RegistryHive::LocalMachine => HKEY_LOCAL_MACHINE,
            RegistryHive::ClassesRoot => HKEY_CLASSES_ROOT,
            RegistryHive::Users => HKEY_USERS,
            RegistryHive::CurrentConfig => HKEY_CURRENT_CONFIG,
        });

        let Ok(key) = root.open_subkey(key_path) else {
            return Ok(None);
        };
        let Ok(value) = key.get_raw_value(value_name) else {
            return Ok(None);
        };

        let mapped = match value.vtype {
            RegType::REG_SZ => RegistryValue::String(decode_utf16_registry_string(&value.bytes)),
            RegType::REG_EXPAND_SZ => {
                RegistryValue::ExpandString(decode_utf16_registry_string(&value.bytes))
            }
            RegType::REG_DWORD => {
                if value.bytes.len() < 4 {
                    return Err(anyhow!("invalid REG_DWORD payload length"));
                }
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&value.bytes[..4]);
                RegistryValue::Dword(u32::from_le_bytes(bytes))
            }
            RegType::REG_QWORD => {
                if value.bytes.len() < 8 {
                    return Err(anyhow!("invalid REG_QWORD payload length"));
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&value.bytes[..8]);
                RegistryValue::Qword(u64::from_le_bytes(bytes))
            }
            RegType::REG_MULTI_SZ => {
                RegistryValue::MultiString(decode_utf16_multi_string(&value.bytes))
            }
            _ => RegistryValue::Binary(value.bytes),
        };

        Ok(Some(mapped))
    }

    fn delete_value(&self, hive: RegistryHive, key_path: &str, value_name: &str) -> Result<()> {
        use winreg::enums::{
            HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
            HKEY_USERS,
        };
        use winreg::RegKey;

        let root = RegKey::predef(match hive {
            RegistryHive::CurrentUser => HKEY_CURRENT_USER,
            RegistryHive::LocalMachine => HKEY_LOCAL_MACHINE,
            RegistryHive::ClassesRoot => HKEY_CLASSES_ROOT,
            RegistryHive::Users => HKEY_USERS,
            RegistryHive::CurrentConfig => HKEY_CURRENT_CONFIG,
        });
        if let Ok(key) = root.open_subkey_with_flags(key_path, winreg::enums::KEY_WRITE) {
            let _ = key.delete_value(value_name);
        }
        Ok(())
    }

    fn delete_key(&self, hive: RegistryHive, key_path: &str) -> Result<()> {
        use winreg::enums::{
            HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
            HKEY_USERS,
        };
        use winreg::RegKey;

        let root = RegKey::predef(match hive {
            RegistryHive::CurrentUser => HKEY_CURRENT_USER,
            RegistryHive::LocalMachine => HKEY_LOCAL_MACHINE,
            RegistryHive::ClassesRoot => HKEY_CLASSES_ROOT,
            RegistryHive::Users => HKEY_USERS,
            RegistryHive::CurrentConfig => HKEY_CURRENT_CONFIG,
        });

        let _ = root.delete_subkey_all(key_path);
        Ok(())
    }

    fn key_exists(&self, hive: RegistryHive, key_path: &str) -> Result<bool> {
        use winreg::enums::{
            HKEY_CLASSES_ROOT, HKEY_CURRENT_CONFIG, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
            HKEY_USERS,
        };
        use winreg::RegKey;

        let root = RegKey::predef(match hive {
            RegistryHive::CurrentUser => HKEY_CURRENT_USER,
            RegistryHive::LocalMachine => HKEY_LOCAL_MACHINE,
            RegistryHive::ClassesRoot => HKEY_CLASSES_ROOT,
            RegistryHive::Users => HKEY_USERS,
            RegistryHive::CurrentConfig => HKEY_CURRENT_CONFIG,
        });

        Ok(root.open_subkey(key_path).is_ok())
    }
}

#[cfg(not(windows))]
impl RegistryBackend for WindowsRegistryBackend {
    fn set_value(
        &self,
        _hive: RegistryHive,
        _key_path: &str,
        _value_name: &str,
        _value: RegistryValue,
    ) -> Result<()> {
        Ok(())
    }

    fn get_value(
        &self,
        _hive: RegistryHive,
        _key_path: &str,
        _value_name: &str,
    ) -> Result<Option<RegistryValue>> {
        Ok(None)
    }

    fn delete_value(&self, _hive: RegistryHive, _key_path: &str, _value_name: &str) -> Result<()> {
        Ok(())
    }

    fn delete_key(&self, _hive: RegistryHive, _key_path: &str) -> Result<()> {
        Ok(())
    }

    fn key_exists(&self, _hive: RegistryHive, _key_path: &str) -> Result<bool> {
        Ok(false)
    }
}

#[cfg(windows)]
fn utf16_bytes_with_double_nul(values: &[String]) -> Vec<u8> {
    let mut utf16 = Vec::<u16>::new();
    for value in values {
        utf16.extend(value.encode_utf16());
        utf16.push(0);
    }
    utf16.push(0);

    let mut bytes = Vec::with_capacity(utf16.len() * 2);
    for chunk in utf16 {
        bytes.extend_from_slice(&chunk.to_le_bytes());
    }
    bytes
}

#[cfg(windows)]
fn decode_utf16_registry_string(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let units = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();
    let mut decoded = String::from_utf16_lossy(&units);
    while decoded.ends_with('\0') {
        decoded.pop();
    }
    decoded
}

#[cfg(windows)]
fn decode_utf16_multi_string(bytes: &[u8]) -> Vec<String> {
    let units = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect::<Vec<_>>();

    let mut values = Vec::new();
    let mut current = Vec::<u16>::new();

    for unit in units {
        if unit == 0 {
            if current.is_empty() {
                break;
            }
            values.push(String::from_utf16_lossy(&current));
            current.clear();
        } else {
            current.push(unit);
        }
    }

    values
}

/// register the `roblox:` and `roblox-player:` protocol handlers.
/// mirrors Ruststrap's WindowsRegistry.RegisterPlayer().
pub fn register_player_protocol(
    backend: &dyn RegistryBackend,
    player_exe: Option<&str>,
    args: Option<&str>,
) -> Result<()> {
    let exe_path = player_exe.unwrap_or("");
    let extra_args = args.unwrap_or("\"%1\"");
    let command = format!("\"{exe_path}\" {extra_args}");

    for proto in &["roblox", "roblox-player"] {
        let base_key = format!("Software\\Classes\\{proto}");
        backend.set_value(
            RegistryHive::CurrentUser,
            &base_key,
            "",
            RegistryValue::String(format!("URL: {proto} Protocol")),
        )?;
        backend.set_value(
            RegistryHive::CurrentUser,
            &base_key,
            "URL Protocol",
            RegistryValue::String(String::new()),
        )?;
        backend.set_value(
            RegistryHive::CurrentUser,
            &format!("{base_key}\\DefaultIcon"),
            "",
            RegistryValue::String(format!("\"{exe_path}\",0")),
        )?;
        backend.set_value(
            RegistryHive::CurrentUser,
            &format!("{base_key}\\shell\\open\\command"),
            "",
            RegistryValue::String(command.clone()),
        )?;
    }

    Ok(())
}

/// register Roblox Studio protocol handlers.
/// mirrors Ruststrap's WindowsRegistry.RegisterStudio().
pub fn register_studio_protocol(
    backend: &dyn RegistryBackend,
    studio_exe: Option<&str>,
    args: Option<&str>,
) -> Result<()> {
    let exe_path = studio_exe.unwrap_or("");
    let extra_args = args.unwrap_or("\"%1\"");
    let command = format!("\"{exe_path}\" {extra_args}");

    for proto in &["roblox-studio", "roblox-studio-auth"] {
        let base_key = format!("Software\\Classes\\{proto}");
        backend.set_value(
            RegistryHive::CurrentUser,
            &base_key,
            "",
            RegistryValue::String(format!("URL: {proto} Protocol")),
        )?;
        backend.set_value(
            RegistryHive::CurrentUser,
            &base_key,
            "URL Protocol",
            RegistryValue::String(String::new()),
        )?;
        backend.set_value(
            RegistryHive::CurrentUser,
            &format!("{base_key}\\shell\\open\\command"),
            "",
            RegistryValue::String(command.clone()),
        )?;
    }

    // file associations
    for ext in &[".rbxl", ".rbxlx"] {
        let ext_key = format!("Software\\Classes\\{ext}");
        backend.set_value(
            RegistryHive::CurrentUser,
            &ext_key,
            "",
            RegistryValue::String("Roblox.Place".to_string()),
        )?;
    }

    let place_key = "Software\\Classes\\Roblox.Place";
    backend.set_value(
        RegistryHive::CurrentUser,
        place_key,
        "",
        RegistryValue::String("Roblox Place".to_string()),
    )?;
    backend.set_value(
        RegistryHive::CurrentUser,
        &format!("{place_key}\\shell\\open\\command"),
        "",
        RegistryValue::String(format!("\"{exe_path}\" -ide \"%1\"")),
    )?;

    Ok(())
}

/// register the client install location in the registry.
/// mirrors Ruststrap's WindowsRegistry.RegisterClientLocation().
pub fn register_client_location(
    backend: &dyn RegistryBackend,
    is_studio: bool,
    version_dir: Option<&str>,
) -> Result<()> {
    let env_key = if is_studio {
        r"Software\ROBLOX Corporation\Environments\roblox-studio"
    } else {
        r"Software\ROBLOX Corporation\Environments\roblox-player"
    };

    if let Some(dir) = version_dir {
        backend.set_value(
            RegistryHive::CurrentUser,
            env_key,
            "",
            RegistryValue::String(dir.to_string()),
        )?;
    } else {
        let _ = backend.delete_key(RegistryHive::CurrentUser, env_key);
    }

    Ok(())
}

/// unregister a protocol handler by name.
pub fn unregister_protocol(backend: &dyn RegistryBackend, proto: &str) -> Result<()> {
    let key = format!("Software\\Classes\\{proto}");
    backend.delete_key(RegistryHive::CurrentUser, &key)
}
