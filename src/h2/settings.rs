// https://www.rfc-editor.org/rfc/rfc9113.html#name-settings
use std::num::NonZeroU32;

/// HTTP/2 Settings.
#[derive(Debug)]
pub struct Settings {
    /// This setting allows the sender to inform the remote endpoint of the maximum size of the
    /// compression table used to decode field blocks, in units of octets.
    pub header_table_size: u32,
    /// This setting can be used to enable or disable server push.
    pub enable_push: bool,
    /// This setting indicates the maximum number of concurrent streams that the sender will allow.
    pub max_concurrent_streams: u32,
    /// This setting indicates the sender's initial window size (in units of octets) for
    /// stream-level flow control
    pub initial_window_size: u32,
    /// This setting indicates the size of the largest frame payload that the sender is willing to
    /// receive, in units of octets.
    pub max_frame_size: u32,
    /// This advisory setting informs a peer of the maximum field section size that the sender is
    /// prepared to accept, in units of octets.
    pub max_header_list_size: Option<NonZeroU32>,
}

/// HTTP/2 defined settings.
#[derive(Clone, Copy, Debug)]
pub enum SettingId {
    /// SETTINGS_HEADER_TABLE_SIZE
    HeaderTableSize = 1,
    /// SETTINGS_ENABLE_PUSH
    EnablePush = 2,
    /// SETTINGS_MAX_CONCURRENT_STREAMS
    MaxConcurrentStreams = 3,
    /// SETTINGS_INITIAL_WINDOW_SIZE
    InitialWindowSize = 4,
    /// SETTINGS_MAX_FRAME_SIZE
    MaxFrameSize = 5,
    /// SETTINGS_MAX_HEADER_LIST_SIZE
    MaxHeaderListSize = 6,
}

impl SettingId {
    /// `SETTINGS_HEADER_TABLE_SIZE`
    pub const HEADER_TABLE_SIZE: u16 = 1;
    /// `SETTINGS_HEADER_ENABLE_PUSH`
    pub const HEADER_ENABLE_PUSH: u16 = 2;
    /// `SETTINGS_MAX_CONCURRENT_STREAMS`
    pub const MAX_CONCURRENT_STREAMS: u16 = 3;
    /// `SETTINGS_INITIAL_WINDOW_SIZE`
    pub const INITIAL_WINDOW_SIZE: u16 = 4;
    /// `SETTINGS_MAX_FRAME_SIZE`
    pub const MAX_FRAME_SIZE: u16 = 5;
    /// `SETTINGS_MAX_HEADER_LIST_SIZE`
    pub const MAX_HEADER_LIST_SIZE: u16 = 6;
}

impl SettingId {
    /// Creates [`SettingId`] from its integer identifier.
    ///
    /// An endpoint that receives a SETTINGS frame with any unknown or unsupported identifier MUST
    /// ignore that setting.
    pub fn from_u16(ty: u16) -> Option<Self> {
        if matches!(ty, 1..7) {
            Some(unsafe { core::mem::transmute::<u8, Self>(ty as u8) })
        } else {
            None
        }
    }
}

impl Settings {
    /// Creates new [`Settings`].
    pub fn new() -> Self {
        Self {
            header_table_size: 4096,
            enable_push: true,
            max_concurrent_streams: 100,
            initial_window_size: 65535,
            max_frame_size: 16384, // hard limit: 16_777_215
            max_header_list_size: None, // default is unlimited
        }
    }

    /// Set setting value by its identifier.
    pub fn set_by_id(&mut self, ident: SettingId, value: u32) {
        match ident {
            SettingId::HeaderTableSize => self.header_table_size = value,
            SettingId::EnablePush => self.enable_push = value != 0,
            SettingId::MaxConcurrentStreams => self.max_concurrent_streams = value,
            SettingId::InitialWindowSize => self.initial_window_size = value,
            SettingId::MaxFrameSize => self.max_frame_size = value,
            SettingId::MaxHeaderListSize => self.max_header_list_size = NonZeroU32::new(value),
        }
    }
}

impl Default for Settings {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ===== Error =====

/// An error that can occur in HTTP/2 settings operation.
///
/// Returning boolean indicating whether given identifier is known.
///
/// An endpoint that receives a SETTINGS frame with any unknown or unsupported identifier MUST
/// ignore that setting.
///
/// Receipt of a SETTINGS frame with the ACK flag set and a length field value other than 0 MUST be
/// treated as a connection error of type FRAME_SIZE_ERROR.
///
/// If an endpoint receives a SETTINGS frame whose Stream Identifier field is anything other than
/// 0x00, the endpoint MUST respond with a connection error of type PROTOCOL_ERROR.
///
/// A badly formed or incomplete SETTINGS frame MUST be treated as a connection error of type
/// PROTOCOL_ERROR.
///
/// A SETTINGS frame with a length other than a multiple of 6 octets MUST be treated as a
/// connection error of type FRAME_SIZE_ERROR.
///
/// Any value of SETTINGS_ENLABE_PUSH other than 0 or 1 MUST be treated as a connection error of
/// type PROTOCOL_ERROR.
///
/// A client MUST treat receipt of a SETTINGS frame with SETTINGS_ENABLE_PUSH set to 1 as a
/// connection error of type PROTOCOL_ERROR.
///
/// Values of SETTINGS_INITIAL_WINDOW_SIZE above the maximum flow-control window size of 231-1 MUST
/// be treated as a connection error of type FLOW_CONTROL_ERROR.
///
/// Values of SETTINGS_MAX_FRAME_SIZE outside this range MUST be treated as a connection error of
/// type PROTOCOL_ERROR.
///
/// If the sender of a SETTINGS frame does not receive an acknowledgment within a reasonable amount
/// of time, it MAY issue a connection error of type SETTINGS_TIMEOUT.
#[derive(Debug)]
pub enum SettingsError {
    /// SETTINGS frame acknowledge with non zero length.
    NonZeroAckLength,
    /// SETTINGS frame with non zero stream id.
    NonZeroId,
    /// Malformed SETTINGS frame.
    Malformed,
    /// A SETTINGS_ENABLE_PUSH value other than 0 or 1.
    NonBoolPushValue,
    /// SETTINGS frame with length that is not multiple of 6.
    FrameSize,
}

impl SettingsError {
    fn message(&self) -> &'static str {
        match self {
            SettingsError::NonZeroAckLength => "non zero ack length settings frame",
            SettingsError::NonZeroId => "non zero identifier settings frame",
            SettingsError::Malformed => "malformed settings frame",
            SettingsError::NonBoolPushValue => "non boolean server push value",
            SettingsError::FrameSize => "invalid settings frame size",
        }
    }
}

impl std::error::Error for SettingsError {}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}
