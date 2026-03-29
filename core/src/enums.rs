use serde::{Deserialize, Serialize};

/// gamejoin types

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameJoinType {
    Unknown,
    RequestGame,
    RequestGameJob,
    RequestPrivateGame,
    RequestFollowUser,
    RequestPlayTogetherGame,
}

impl Default for GameJoinType {
    fn default() -> Self {
        Self::Unknown
    }
}

// section: servertype

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerType {
    Public,
    Private,
    Reserved,
}

impl Default for ServerType {
    fn default() -> Self {
        Self::Public
    }
}

impl ServerType {
    pub fn display_string(&self) -> &'static str {
        match self {
            ServerType::Public => "Public",
            ServerType::Private => "Private",
            ServerType::Reserved => "Reserved",
        }
    }
}

// cursor type

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CursorType {
    Default,
    From2006,
    From2013,
}

impl Default for CursorType {
    fn default() -> Self {
        Self::Default
    }
}

// emoji type

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmojiType {
    Default,
    Disabled,
    SystemDefault,
}

impl Default for EmojiType {
    fn default() -> Self {
        Self::Default
    }
}

// section: robloxicon
// this is like so repititive I should just copy and paste and change the names lmao

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobloxIcon {
    IconDefault,
    Icon2017,
    Icon2019,
    Icon2022,
    IconEarly2023,
    IconLate2023,
}

impl Default for RobloxIcon {
    fn default() -> Self {
        Self::IconDefault
    }
}

// section: generictristate

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenericTriState {
    Unknown,
    Successful,
    Failed,
}

impl Default for GenericTriState {
    fn default() -> Self {
        Self::Unknown
    }
}

// section: errorcode

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ErrorCode {
    ErrorSuccess = 0,
    ErrorInvalidFunction = 1,
    ErrorFileNotFound = 2,
    ErrorInstallUserexit = 1602,
    ErrorInstallFailure = 1603,
    ErrorCancelled = 1223,
}

impl Default for ErrorCode {
    fn default() -> Self {
        Self::ErrorSuccess
    }
}

// section: nextaction

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NextAction {
    Terminate,
    LaunchSettings,
    LaunchRoblox,
    LaunchRobloxStudio,
}

impl Default for NextAction {
    fn default() -> Self {
        Self::Terminate
    }
}

// section: theme

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    Default,
    Light,
    Dark,
}

impl Default for Theme {
    fn default() -> Self {
        Self::Default
    }
}

// section: customthemetemplate

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomThemeTemplate {
    Default,
    Transparent,
    BlurBehind,
    Acrylic,
    Mica,
}

impl Default for CustomThemeTemplate {
    fn default() -> Self {
        Self::Default
    }
}

// section: webenvironment

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebEnvironment {
    Production,
    GameTest1,
    GameTest2,
    GameTest3,
}

impl Default for WebEnvironment {
    fn default() -> Self {
        Self::Production
    }
}

// section: renderingmode (flagpreset)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderingMode {
    Default,
    Vulkan,
    D3D11,
    D3D11FL10,
    OpenGL,
}

impl Default for RenderingMode {
    fn default() -> Self {
        Self::Default
    }
}

// section: msaamode (flagpreset)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MSAAMode {
    Default,
    X1,
    X2,
    X4,
}

impl Default for MSAAMode {
    fn default() -> Self {
        Self::Default
    }
}

// section: texturequality (flagpreset)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextureQuality {
    Default,
    Level0,
    Level1,
    Level2,
    Level3,
}

impl Default for TextureQuality {
    fn default() -> Self {
        Self::Default
    }
}

// section: fontsize (gbspreset)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontSize {
    Default,
    X1,
    X2,
    X3,
    X4,
}

impl Default for FontSize {
    fn default() -> Self {
        Self::Default
    }
}

impl FontSize {
    pub fn xml_value(&self) -> Option<&'static str> {
        match self {
            FontSize::Default => None,
            FontSize::X1 => Some("1"),
            FontSize::X2 => Some("2"),
            FontSize::X3 => Some("3"),
            FontSize::X4 => Some("4"),
        }
    }
}

// section: versioncomparison

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionComparison {
    Greater,
    Equal,
    Less,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_defaults() {
        assert_eq!(GameJoinType::default(), GameJoinType::Unknown);
        assert_eq!(ServerType::default(), ServerType::Public);
        assert_eq!(GenericTriState::default(), GenericTriState::Unknown);
        assert_eq!(ErrorCode::default(), ErrorCode::ErrorSuccess);
        assert_eq!(NextAction::default(), NextAction::Terminate);
    }

    #[test]
    fn server_type_display() {
        assert_eq!(ServerType::Private.display_string(), "Private");
        assert_eq!(ServerType::Reserved.display_string(), "Reserved");
    }

    #[test]
    fn font_size_xml_value() {
        assert_eq!(FontSize::Default.xml_value(), None);
        assert_eq!(FontSize::X2.xml_value(), Some("2"));
    }
}
