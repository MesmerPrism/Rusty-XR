//! Open Sound Control packet and UDP helpers for Rusty XR.
//!
//! This crate implements a small OSC 1.0-compatible message/bundle codec plus
//! a standard-library UDP wrapper. It is intended for live control and sensor
//! ingress between operator tools, phones, companion apps, and headset app
//! shells. It does not define app-specific address trees or sensor semantics.
//!
//! Enable the `serde` feature when OSC endpoint descriptors or decoded packets
//! need to cross process boundaries.
//!
//! ```
//! use rusty_xr_osc::{decode_packet, encode_packet, OscArgument, OscMessage, OscPacket};
//!
//! let packet = OscPacket::Message(
//!     OscMessage::new("/rusty-xr/probe")
//!         .expect("address is valid")
//!         .with_argument(OscArgument::Float(0.5)),
//! );
//! let encoded = encode_packet(&packet).expect("packet should encode");
//! assert_eq!(decode_packet(&encoded).expect("packet should decode"), packet);
//! ```

use std::{
    fmt,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::{Duration, SystemTime},
};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// OSC timetag value meaning "immediately".
pub const OSC_TIMETAG_IMMEDIATE: u64 = 1;

/// Default maximum UDP datagram size accepted by the helper receiver.
pub const DEFAULT_MAX_PACKET_BYTES: usize = 65_507;

/// Generic OSC stream role used by public examples and adapters.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OscStreamRole {
    SensorIngress,
    RuntimeCommand,
    Probe,
    Custom,
}

/// Public UDP endpoint configuration for an OSC receiver or sender.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OscEndpointConfig {
    pub bind_addr: String,
    pub peer_addr: Option<String>,
    pub max_packet_bytes: usize,
    pub role: OscStreamRole,
}

impl OscEndpointConfig {
    pub fn new(bind_addr: impl Into<String>) -> Self {
        Self {
            bind_addr: bind_addr.into(),
            peer_addr: None,
            max_packet_bytes: DEFAULT_MAX_PACKET_BYTES,
            role: OscStreamRole::SensorIngress,
        }
    }

    pub fn with_peer_addr(mut self, peer_addr: impl Into<String>) -> Self {
        self.peer_addr = Some(peer_addr.into());
        self
    }

    pub fn with_max_packet_bytes(mut self, max_packet_bytes: usize) -> Self {
        self.max_packet_bytes = max_packet_bytes.clamp(256, DEFAULT_MAX_PACKET_BYTES);
        self
    }

    pub const fn with_role(mut self, role: OscStreamRole) -> Self {
        self.role = role;
        self
    }

    pub fn is_valid(&self) -> bool {
        !self.bind_addr.trim().is_empty()
            && self.max_packet_bytes > 0
            && self.max_packet_bytes <= DEFAULT_MAX_PACKET_BYTES
    }
}

/// Lightweight endpoint status suitable for diagnostics and operator tools.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OscEndpointStatus {
    pub config: OscEndpointConfig,
    pub local_addr: Option<String>,
    pub packet_count: u64,
    pub last_packet_time_ns: Option<u64>,
    pub last_peer_addr: Option<String>,
    pub last_error: Option<String>,
}

impl OscEndpointStatus {
    pub fn new(config: OscEndpointConfig) -> Self {
        Self {
            config,
            local_addr: None,
            packet_count: 0,
            last_packet_time_ns: None,
            last_peer_addr: None,
            last_error: None,
        }
    }

    pub fn record_packet(&mut self, peer_addr: SocketAddr, received_time_ns: u64) {
        self.packet_count = self.packet_count.saturating_add(1);
        self.last_packet_time_ns = Some(received_time_ns);
        self.last_peer_addr = Some(peer_addr.to_string());
        self.last_error = None;
    }

    pub fn record_error(&mut self, error: impl Into<String>) {
        self.last_error = Some(error.into());
    }

    pub fn packet_age_ns(&self, now_ns: u64) -> Option<u64> {
        now_ns.checked_sub(self.last_packet_time_ns?)
    }
}

/// OSC packet tree.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum OscPacket {
    Message(OscMessage),
    Bundle(OscBundle),
}

impl OscPacket {
    pub fn message(address: impl Into<String>) -> Result<Self, OscError> {
        Ok(Self::Message(OscMessage::new(address)?))
    }
}

/// OSC message with a slash-prefixed address pattern.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct OscMessage {
    pub address: String,
    pub arguments: Vec<OscArgument>,
}

impl OscMessage {
    pub fn new(address: impl Into<String>) -> Result<Self, OscError> {
        let address = address.into();
        validate_address(&address)?;
        Ok(Self {
            address,
            arguments: Vec::new(),
        })
    }

    pub fn with_argument(mut self, argument: OscArgument) -> Self {
        self.arguments.push(argument);
        self
    }

    pub fn type_tag_string(&self) -> String {
        let mut tags = String::from(",");
        for argument in &self.arguments {
            tags.push(argument.type_tag());
        }
        tags
    }
}

/// OSC bundle with an NTP-style timetag and nested packets.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct OscBundle {
    pub timetag: u64,
    pub packets: Vec<OscPacket>,
}

impl OscBundle {
    pub fn immediate(packets: Vec<OscPacket>) -> Self {
        Self {
            timetag: OSC_TIMETAG_IMMEDIATE,
            packets,
        }
    }
}

/// Common OSC argument types.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum OscArgument {
    Int(i32),
    Float(f32),
    String(String),
    Blob(Vec<u8>),
    Bool(bool),
    Nil,
    Impulse,
}

impl OscArgument {
    pub const fn type_tag(&self) -> char {
        match self {
            Self::Int(_) => 'i',
            Self::Float(_) => 'f',
            Self::String(_) => 's',
            Self::Blob(_) => 'b',
            Self::Bool(true) => 'T',
            Self::Bool(false) => 'F',
            Self::Nil => 'N',
            Self::Impulse => 'I',
        }
    }
}

/// Decoded UDP OSC datagram with sender metadata.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct OscReceivedPacket {
    pub packet: OscPacket,
    pub peer_addr: String,
    pub byte_len: usize,
    pub received_time_ns: u64,
}

/// OSC parse/encode errors.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OscError {
    InvalidAddress(String),
    UnexpectedEof,
    MissingStringTerminator,
    InvalidUtf8,
    InvalidTypeTagString(String),
    UnsupportedTypeTag(char),
    InvalidBundleElementSize(i32),
    TrailingBytes(usize),
    PacketTooLarge(usize),
}

impl fmt::Display for OscError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAddress(value) => write!(f, "invalid OSC address: {value}"),
            Self::UnexpectedEof => f.write_str("unexpected end of OSC packet"),
            Self::MissingStringTerminator => f.write_str("OSC string is missing a null terminator"),
            Self::InvalidUtf8 => f.write_str("OSC string is not valid UTF-8"),
            Self::InvalidTypeTagString(value) => write!(f, "invalid OSC type tag string: {value}"),
            Self::UnsupportedTypeTag(tag) => write!(f, "unsupported OSC type tag: {tag}"),
            Self::InvalidBundleElementSize(size) => {
                write!(f, "invalid OSC bundle element size: {size}")
            }
            Self::TrailingBytes(count) => write!(f, "OSC packet has {count} trailing bytes"),
            Self::PacketTooLarge(size) => write!(f, "OSC packet is too large: {size} bytes"),
        }
    }
}

impl std::error::Error for OscError {}

/// UDP helper errors.
#[derive(Debug)]
pub enum OscIoError {
    Codec(OscError),
    Io(std::io::Error),
    MissingPeerAddress,
    AddressResolutionFailed(String),
}

impl fmt::Display for OscIoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Codec(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "{error}"),
            Self::MissingPeerAddress => f.write_str("OSC endpoint has no peer address"),
            Self::AddressResolutionFailed(address) => {
                write!(f, "could not resolve OSC address: {address}")
            }
        }
    }
}

impl std::error::Error for OscIoError {}

impl From<OscError> for OscIoError {
    fn from(value: OscError) -> Self {
        Self::Codec(value)
    }
}

impl From<std::io::Error> for OscIoError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// Standard-library UDP OSC endpoint.
pub struct OscUdpSocket {
    socket: UdpSocket,
    config: OscEndpointConfig,
}

impl OscUdpSocket {
    pub fn bind(config: OscEndpointConfig) -> Result<Self, OscIoError> {
        let socket = UdpSocket::bind(&config.bind_addr)?;
        Ok(Self { socket, config })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, OscIoError> {
        Ok(self.socket.local_addr()?)
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), OscIoError> {
        Ok(self.socket.set_read_timeout(timeout)?)
    }

    pub fn send_packet(&self, packet: &OscPacket) -> Result<usize, OscIoError> {
        let peer_addr = self
            .config
            .peer_addr
            .as_deref()
            .ok_or(OscIoError::MissingPeerAddress)?;
        self.send_packet_to(packet, peer_addr)
    }

    pub fn send_packet_to(
        &self,
        packet: &OscPacket,
        peer_addr: impl ToSocketAddrs,
    ) -> Result<usize, OscIoError> {
        let encoded = encode_packet(packet)?;
        if encoded.len() > self.config.max_packet_bytes {
            return Err(OscError::PacketTooLarge(encoded.len()).into());
        }
        Ok(self.socket.send_to(&encoded, peer_addr)?)
    }

    pub fn recv_packet(&self) -> Result<OscReceivedPacket, OscIoError> {
        let mut buffer = vec![0_u8; self.config.max_packet_bytes];
        let (byte_len, peer_addr) = self.socket.recv_from(&mut buffer)?;
        let packet = decode_packet(&buffer[..byte_len])?;
        Ok(OscReceivedPacket {
            packet,
            peer_addr: peer_addr.to_string(),
            byte_len,
            received_time_ns: system_time_ns(SystemTime::now()),
        })
    }
}

/// Encodes a complete OSC packet.
pub fn encode_packet(packet: &OscPacket) -> Result<Vec<u8>, OscError> {
    let mut out = Vec::new();
    encode_packet_into(packet, &mut out)?;
    Ok(out)
}

/// Decodes a complete OSC packet.
pub fn decode_packet(bytes: &[u8]) -> Result<OscPacket, OscError> {
    let (packet, offset) = decode_packet_at(bytes, 0, bytes.len())?;
    if offset != bytes.len() {
        return Err(OscError::TrailingBytes(bytes.len() - offset));
    }
    Ok(packet)
}

fn encode_packet_into(packet: &OscPacket, out: &mut Vec<u8>) -> Result<(), OscError> {
    match packet {
        OscPacket::Message(message) => encode_message(message, out),
        OscPacket::Bundle(bundle) => encode_bundle(bundle, out),
    }
}

fn encode_message(message: &OscMessage, out: &mut Vec<u8>) -> Result<(), OscError> {
    validate_address(&message.address)?;
    push_padded_string(out, &message.address);
    push_padded_string(out, &message.type_tag_string());
    for argument in &message.arguments {
        match argument {
            OscArgument::Int(value) => out.extend(value.to_be_bytes()),
            OscArgument::Float(value) => out.extend(value.to_bits().to_be_bytes()),
            OscArgument::String(value) => push_padded_string(out, value),
            OscArgument::Blob(value) => {
                let len = i32::try_from(value.len())
                    .map_err(|_| OscError::PacketTooLarge(value.len()))?;
                out.extend(len.to_be_bytes());
                out.extend(value);
                push_zero_padding(out, value.len());
            }
            OscArgument::Bool(_) | OscArgument::Nil | OscArgument::Impulse => {}
        }
    }
    Ok(())
}

fn encode_bundle(bundle: &OscBundle, out: &mut Vec<u8>) -> Result<(), OscError> {
    push_padded_string(out, "#bundle");
    out.extend(bundle.timetag.to_be_bytes());
    for packet in &bundle.packets {
        let encoded = encode_packet(packet)?;
        let size =
            i32::try_from(encoded.len()).map_err(|_| OscError::PacketTooLarge(encoded.len()))?;
        out.extend(size.to_be_bytes());
        out.extend(encoded);
    }
    Ok(())
}

fn decode_packet_at(
    bytes: &[u8],
    offset: usize,
    limit: usize,
) -> Result<(OscPacket, usize), OscError> {
    let (header, next) = read_padded_string(bytes, offset, limit)?;
    if header == "#bundle" {
        return decode_bundle(bytes, next, limit);
    }

    validate_address(&header)?;
    let (type_tags, mut cursor) = read_padded_string(bytes, next, limit)?;
    if !type_tags.starts_with(',') {
        return Err(OscError::InvalidTypeTagString(type_tags));
    }

    let mut arguments = Vec::new();
    for tag in type_tags[1..].chars() {
        let argument = match tag {
            'i' => {
                let value = read_i32(bytes, cursor, limit)?;
                cursor += 4;
                OscArgument::Int(value)
            }
            'f' => {
                let bits = read_u32(bytes, cursor, limit)?;
                cursor += 4;
                OscArgument::Float(f32::from_bits(bits))
            }
            's' => {
                let (value, next) = read_padded_string(bytes, cursor, limit)?;
                cursor = next;
                OscArgument::String(value)
            }
            'b' => {
                let size = read_i32(bytes, cursor, limit)?;
                cursor += 4;
                if size < 0 {
                    return Err(OscError::InvalidBundleElementSize(size));
                }
                let size = size as usize;
                if cursor + size > limit {
                    return Err(OscError::UnexpectedEof);
                }
                let blob = bytes[cursor..cursor + size].to_vec();
                cursor += padded_len(size);
                if cursor > limit {
                    return Err(OscError::UnexpectedEof);
                }
                OscArgument::Blob(blob)
            }
            'T' => OscArgument::Bool(true),
            'F' => OscArgument::Bool(false),
            'N' => OscArgument::Nil,
            'I' => OscArgument::Impulse,
            other => return Err(OscError::UnsupportedTypeTag(other)),
        };
        arguments.push(argument);
    }

    Ok((
        OscPacket::Message(OscMessage {
            address: header,
            arguments,
        }),
        cursor,
    ))
}

fn decode_bundle(
    bytes: &[u8],
    offset: usize,
    limit: usize,
) -> Result<(OscPacket, usize), OscError> {
    if offset + 8 > limit {
        return Err(OscError::UnexpectedEof);
    }
    let mut timetag_bytes = [0_u8; 8];
    timetag_bytes.copy_from_slice(&bytes[offset..offset + 8]);
    let timetag = u64::from_be_bytes(timetag_bytes);
    let mut cursor = offset + 8;
    let mut packets = Vec::new();
    while cursor < limit {
        let size = read_i32(bytes, cursor, limit)?;
        cursor += 4;
        if size <= 0 {
            return Err(OscError::InvalidBundleElementSize(size));
        }
        let size = size as usize;
        let element_limit = cursor + size;
        if element_limit > limit {
            return Err(OscError::UnexpectedEof);
        }
        let (packet, next) = decode_packet_at(bytes, cursor, element_limit)?;
        if next != element_limit {
            return Err(OscError::TrailingBytes(element_limit - next));
        }
        packets.push(packet);
        cursor = element_limit;
    }
    Ok((OscPacket::Bundle(OscBundle { timetag, packets }), cursor))
}

fn validate_address(address: &str) -> Result<(), OscError> {
    if !address.starts_with('/') || address.as_bytes().contains(&0) {
        return Err(OscError::InvalidAddress(address.to_string()));
    }
    Ok(())
}

fn push_padded_string(out: &mut Vec<u8>, value: &str) {
    out.extend(value.as_bytes());
    out.push(0);
    push_zero_padding(out, value.len() + 1);
}

fn push_zero_padding(out: &mut Vec<u8>, unpadded_len: usize) {
    let padding = (4 - (unpadded_len % 4)) % 4;
    out.extend(std::iter::repeat_n(0, padding));
}

fn padded_len(value: usize) -> usize {
    value + ((4 - (value % 4)) % 4)
}

fn read_padded_string(
    bytes: &[u8],
    offset: usize,
    limit: usize,
) -> Result<(String, usize), OscError> {
    if offset >= limit {
        return Err(OscError::UnexpectedEof);
    }
    let mut cursor = offset;
    while cursor < limit && bytes[cursor] != 0 {
        cursor += 1;
    }
    if cursor >= limit {
        return Err(OscError::MissingStringTerminator);
    }
    let value = std::str::from_utf8(&bytes[offset..cursor])
        .map_err(|_| OscError::InvalidUtf8)?
        .to_string();
    let next = offset + padded_len(cursor - offset + 1);
    if next > limit {
        return Err(OscError::UnexpectedEof);
    }
    Ok((value, next))
}

fn read_i32(bytes: &[u8], offset: usize, limit: usize) -> Result<i32, OscError> {
    if offset + 4 > limit {
        return Err(OscError::UnexpectedEof);
    }
    let mut data = [0_u8; 4];
    data.copy_from_slice(&bytes[offset..offset + 4]);
    Ok(i32::from_be_bytes(data))
}

fn read_u32(bytes: &[u8], offset: usize, limit: usize) -> Result<u32, OscError> {
    if offset + 4 > limit {
        return Err(OscError::UnexpectedEof);
    }
    let mut data = [0_u8; 4];
    data.copy_from_slice(&bytes[offset..offset + 4]);
    Ok(u32::from_be_bytes(data))
}

fn system_time_ns(time: SystemTime) -> u64 {
    time.duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_nanos().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn resolve_addr(address: &str) -> Result<SocketAddr, OscIoError> {
    address
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| OscIoError::AddressResolutionFailed(address.to_string()))
}

/// Sends one OSC packet from an ephemeral UDP socket.
pub fn send_packet_to(packet: &OscPacket, peer_addr: &str) -> Result<usize, OscIoError> {
    let peer = resolve_addr(peer_addr)?;
    let bind_addr = if peer.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let socket = OscUdpSocket::bind(OscEndpointConfig::new(bind_addr).with_peer_addr(peer_addr))?;
    socket.send_packet(packet)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_message() -> OscMessage {
        OscMessage::new("/rusty-xr/probe")
            .expect("address should be valid")
            .with_argument(OscArgument::Int(7))
            .with_argument(OscArgument::Float(0.25))
            .with_argument(OscArgument::String("hello".to_string()))
            .with_argument(OscArgument::Blob(vec![1, 2, 3]))
            .with_argument(OscArgument::Bool(true))
            .with_argument(OscArgument::Bool(false))
            .with_argument(OscArgument::Nil)
            .with_argument(OscArgument::Impulse)
    }

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn validates_endpoint_config() {
        let config = OscEndpointConfig::new("0.0.0.0:9000")
            .with_peer_addr("127.0.0.1:9001")
            .with_role(OscStreamRole::Probe);

        assert!(config.is_valid());
        assert_eq!(config.peer_addr.as_deref(), Some("127.0.0.1:9001"));
    }

    #[test]
    fn validates_message_addresses() {
        assert!(OscMessage::new("/valid").is_ok());
        assert!(OscMessage::new("invalid").is_err());
        assert!(OscMessage::new("/bad\0address").is_err());
    }

    #[test]
    fn encodes_and_decodes_message() {
        let packet = OscPacket::Message(sample_message());

        let encoded = encode_packet(&packet).expect("message should encode");
        let decoded = decode_packet(&encoded).expect("message should decode");

        assert_eq!(decoded, packet);
    }

    #[test]
    fn encodes_and_decodes_bundle() {
        let packet = OscPacket::Bundle(OscBundle::immediate(vec![OscPacket::Message(
            sample_message(),
        )]));

        let encoded = encode_packet(&packet).expect("bundle should encode");
        let decoded = decode_packet(&encoded).expect("bundle should decode");

        assert_eq!(decoded, packet);
    }

    #[test]
    fn rejects_unsupported_type_tags() {
        let mut encoded = Vec::new();
        push_padded_string(&mut encoded, "/rusty-xr/probe");
        push_padded_string(&mut encoded, ",d");
        encoded.extend(1.0_f64.to_bits().to_be_bytes());

        assert_eq!(
            decode_packet(&encoded),
            Err(OscError::UnsupportedTypeTag('d'))
        );
    }

    #[test]
    fn udp_loopback_receives_packet() {
        let receiver = OscUdpSocket::bind(
            OscEndpointConfig::new("127.0.0.1:0").with_role(OscStreamRole::Probe),
        )
        .expect("receiver should bind");
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .expect("timeout should set");
        let target = receiver.local_addr().expect("receiver should have addr");
        let packet = OscPacket::Message(sample_message());

        send_packet_to(&packet, &target.to_string()).expect("packet should send");
        let received = receiver.recv_packet().expect("packet should arrive");

        assert_eq!(received.packet, packet);
        assert!(received.byte_len > 0);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn received_packet_round_trips_with_serde() {
        let packet = OscReceivedPacket {
            packet: OscPacket::Message(sample_message()),
            peer_addr: "127.0.0.1:9000".to_string(),
            byte_len: 64,
            received_time_ns: 100,
        };

        let encoded = serde_json::to_string(&packet).expect("packet should serialize");
        let decoded: OscReceivedPacket =
            serde_json::from_str(&encoded).expect("packet should deserialize");

        assert_eq!(decoded, packet);
    }
}
