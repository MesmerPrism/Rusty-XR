//! Polar H10 data contracts and protocol helpers for Rusty XR.
//!
//! This crate is an independent, pure-Rust model layer. It does not open BLE
//! devices, request Android permissions, link against Polar SDKs, or publish LSL
//! streams directly.
//!
//! Enable the `serde` feature to serialize decoded public Polar payloads and
//! derived stream metadata. Transport adapters remain outside this crate.

use rusty_xr_ble::{BleUuid, GattCharacteristicPath};
use rusty_xr_lsl::{
    LslChannelFormat, LslChannelSchema, LslStreamDescriptor, LslStreamRole, POLAR_ACC_STREAM_TYPE,
    POLAR_ECG_STREAM_TYPE, POLAR_HEART_RATE_STREAM_TYPE,
};

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Standard Heart Rate Service UUID.
pub const HEART_RATE_SERVICE_UUID: &str = "0000180d-0000-1000-8000-00805f9b34fb";

/// Standard Heart Rate Measurement characteristic UUID.
pub const HEART_RATE_MEASUREMENT_UUID: &str = "00002a37-0000-1000-8000-00805f9b34fb";

/// Standard Battery Service UUID.
pub const BATTERY_SERVICE_UUID: &str = "0000180f-0000-1000-8000-00805f9b34fb";

/// Standard Battery Level characteristic UUID.
pub const BATTERY_LEVEL_UUID: &str = "00002a19-0000-1000-8000-00805f9b34fb";

/// Polar Measurement Data service UUID.
pub const PMD_SERVICE_UUID: &str = "fb005c80-02e7-f387-1cad-8acd2d8df0c8";

/// Polar PMD control point characteristic UUID.
pub const PMD_CONTROL_POINT_UUID: &str = "fb005c81-02e7-f387-1cad-8acd2d8df0c8";

/// Polar PMD data characteristic UUID.
pub const PMD_DATA_UUID: &str = "fb005c82-02e7-f387-1cad-8acd2d8df0c8";

/// Polar PMD ECG measurement type.
pub const PMD_MEASUREMENT_TYPE_ECG: u8 = 0x00;

/// Polar PMD ACC measurement type.
pub const PMD_MEASUREMENT_TYPE_ACC: u8 = 0x02;

/// Polar PMD sample-rate setting type.
pub const PMD_SETTING_TYPE_SAMPLE_RATE: u8 = 0x00;

/// Polar PMD resolution setting type.
pub const PMD_SETTING_TYPE_RESOLUTION: u8 = 0x01;

/// Polar PMD range setting type.
pub const PMD_SETTING_TYPE_RANGE: u8 = 0x02;

/// Polar PMD channel-count setting type.
pub const PMD_SETTING_TYPE_CHANNELS: u8 = 0x04;

const PMD_OPCODE_GET_SETTINGS: u8 = 0x01;
const PMD_OPCODE_START_STREAM: u8 = 0x02;
const PMD_OPCODE_STOP_STREAM: u8 = 0x03;
const PMD_HEADER_SIZE: usize = 10;
const ECG_BYTES_PER_SAMPLE: usize = 3;
const ACC_BYTES_PER_UNCOMPRESSED_SAMPLE: usize = 6;

/// Common Polar H10 GATT characteristic paths.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolarGattPaths {
    pub heart_rate_measurement: GattCharacteristicPath,
    pub pmd_control_point: GattCharacteristicPath,
    pub pmd_data: GattCharacteristicPath,
    pub battery_level: GattCharacteristicPath,
}

impl Default for PolarGattPaths {
    fn default() -> Self {
        Self {
            heart_rate_measurement: GattCharacteristicPath::new(
                BleUuid::new(HEART_RATE_SERVICE_UUID),
                BleUuid::new(HEART_RATE_MEASUREMENT_UUID),
            ),
            pmd_control_point: GattCharacteristicPath::new(
                BleUuid::new(PMD_SERVICE_UUID),
                BleUuid::new(PMD_CONTROL_POINT_UUID),
            ),
            pmd_data: GattCharacteristicPath::new(
                BleUuid::new(PMD_SERVICE_UUID),
                BleUuid::new(PMD_DATA_UUID),
            ),
            battery_level: GattCharacteristicPath::new(
                BleUuid::new(BATTERY_SERVICE_UUID),
                BleUuid::new(BATTERY_LEVEL_UUID),
            ),
        }
    }
}

/// Decoding error for Polar H10 payloads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PolarDecodeError {
    EmptyPayload,
    PayloadTooShort {
        expected_at_least: usize,
        actual: usize,
    },
    UnexpectedMeasurementType {
        expected: u8,
        actual: u8,
    },
    BadFrameLength {
        frame: &'static str,
        payload_length: usize,
        bytes_per_sample: usize,
    },
    UnsupportedFrameType(u8),
}

/// Standard BLE Heart Rate Measurement contact status.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeartRateSensorContact {
    NotSupported,
    SupportedNotDetected,
    SupportedDetected,
    Unknown,
}

/// Decoded heart-rate and RR-interval sample.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct PolarHeartRateReading {
    pub bpm: u16,
    pub rr_intervals_ms: Vec<f32>,
    pub energy_expended: Option<u16>,
    pub sensor_contact: HeartRateSensorContact,
}

impl PolarHeartRateReading {
    /// Returns true when the sample includes at least one RR interval.
    pub fn has_rr_intervals(&self) -> bool {
        !self.rr_intervals_ms.is_empty()
    }
}

/// Decode the standard BLE Heart Rate Measurement characteristic payload.
pub fn decode_heart_rate_measurement(
    payload: &[u8],
) -> Result<PolarHeartRateReading, PolarDecodeError> {
    if payload.is_empty() {
        return Err(PolarDecodeError::EmptyPayload);
    }
    if payload.len() < 2 {
        return Err(PolarDecodeError::PayloadTooShort {
            expected_at_least: 2,
            actual: payload.len(),
        });
    }

    let flags = payload[0];
    let heart_rate_is_u16 = (flags & 0x01) != 0;
    let sensor_contact = decode_sensor_contact(flags);
    let mut offset = 1;
    let bpm = if heart_rate_is_u16 {
        let bytes = read_le_u16(payload, offset)?;
        offset += 2;
        bytes
    } else {
        let value = payload[offset] as u16;
        offset += 1;
        value
    };

    let energy_expended = if (flags & 0x08) != 0 {
        let energy = read_le_u16(payload, offset)?;
        offset += 2;
        Some(energy)
    } else {
        None
    };

    let rr_intervals_ms = if (flags & 0x10) != 0 {
        decode_rr_intervals(payload, offset)?
    } else {
        Vec::new()
    };

    Ok(PolarHeartRateReading {
        bpm,
        rr_intervals_ms,
        energy_expended,
        sensor_contact,
    })
}

/// Decoded Polar ECG PMD frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolarEcgFrame {
    pub sensor_timestamp_ns: u64,
    pub samples_microvolts: Vec<i32>,
}

/// A single accelerometer sample in milli-g.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolarAccSample {
    pub x_mg: i16,
    pub y_mg: i16,
    pub z_mg: i16,
}

impl PolarAccSample {
    /// Converts milli-g values to fractional g values.
    pub fn to_g(self) -> [f32; 3] {
        [
            self.x_mg as f32 * 0.001,
            self.y_mg as f32 * 0.001,
            self.z_mg as f32 * 0.001,
        ]
    }
}

/// Decoded Polar ACC PMD frame.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolarAccFrame {
    pub sensor_timestamp_ns: u64,
    pub samples_mg: Vec<PolarAccSample>,
}

/// Decode an uncompressed Polar ECG PMD frame.
pub fn decode_ecg_pmd_frame(payload: &[u8]) -> Result<PolarEcgFrame, PolarDecodeError> {
    validate_pmd_header(payload, PMD_MEASUREMENT_TYPE_ECG)?;
    let frame_type = payload[9];
    if frame_type != 0x00 {
        return Err(PolarDecodeError::UnsupportedFrameType(frame_type));
    }

    let data = &payload[PMD_HEADER_SIZE..];
    if !data.len().is_multiple_of(ECG_BYTES_PER_SAMPLE) {
        return Err(PolarDecodeError::BadFrameLength {
            frame: "ecg",
            payload_length: data.len(),
            bytes_per_sample: ECG_BYTES_PER_SAMPLE,
        });
    }

    let samples_microvolts = data
        .chunks_exact(ECG_BYTES_PER_SAMPLE)
        .map(read_i24_le)
        .collect();

    Ok(PolarEcgFrame {
        sensor_timestamp_ns: read_pmd_timestamp_ns(payload)?,
        samples_microvolts,
    })
}

/// Decode an uncompressed Polar ACC PMD frame.
///
/// Compressed ACC deltas are intentionally not decoded yet; this helper keeps
/// the public contract small and deterministic until the compressed format is
/// covered by independent tests.
pub fn decode_uncompressed_acc_pmd_frame(
    payload: &[u8],
) -> Result<PolarAccFrame, PolarDecodeError> {
    validate_pmd_header(payload, PMD_MEASUREMENT_TYPE_ACC)?;
    let frame_type = payload[9];
    if frame_type != 0x01 {
        return Err(PolarDecodeError::UnsupportedFrameType(frame_type));
    }

    let data = &payload[PMD_HEADER_SIZE..];
    if !data.len().is_multiple_of(ACC_BYTES_PER_UNCOMPRESSED_SAMPLE) {
        return Err(PolarDecodeError::BadFrameLength {
            frame: "acc",
            payload_length: data.len(),
            bytes_per_sample: ACC_BYTES_PER_UNCOMPRESSED_SAMPLE,
        });
    }

    let samples_mg = data
        .chunks_exact(ACC_BYTES_PER_UNCOMPRESSED_SAMPLE)
        .map(|chunk| PolarAccSample {
            x_mg: i16::from_le_bytes([chunk[0], chunk[1]]),
            y_mg: i16::from_le_bytes([chunk[2], chunk[3]]),
            z_mg: i16::from_le_bytes([chunk[4], chunk[5]]),
        })
        .collect();

    Ok(PolarAccFrame {
        sensor_timestamp_ns: read_pmd_timestamp_ns(payload)?,
        samples_mg,
    })
}

/// PMD stream kind supported by the public helpers.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolarPmdStreamKind {
    Ecg,
    Acc,
}

impl PolarPmdStreamKind {
    /// Polar PMD measurement type byte.
    pub const fn measurement_type(self) -> u8 {
        match self {
            Self::Ecg => PMD_MEASUREMENT_TYPE_ECG,
            Self::Acc => PMD_MEASUREMENT_TYPE_ACC,
        }
    }
}

/// PMD stream settings used when building start commands.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PolarPmdStreamSettings {
    pub sample_rate_hz: u16,
    pub resolution_bits: u16,
    pub range_g: Option<u16>,
    pub channels: Option<u8>,
}

impl PolarPmdStreamSettings {
    /// H10 ECG defaults observed in public PolarH10 integration docs.
    pub const fn h10_ecg_default() -> Self {
        Self {
            sample_rate_hz: 130,
            resolution_bits: 14,
            range_g: None,
            channels: None,
        }
    }

    /// H10 accelerometer defaults observed in public PolarH10 integration docs.
    pub const fn h10_acc_default() -> Self {
        Self {
            sample_rate_hz: 200,
            resolution_bits: 16,
            range_g: Some(8),
            channels: None,
        }
    }
}

/// Build a PMD "get settings" control-point command.
pub fn build_get_settings_request(kind: PolarPmdStreamKind) -> Vec<u8> {
    vec![PMD_OPCODE_GET_SETTINGS, kind.measurement_type()]
}

/// Build a PMD "stop stream" control-point command.
pub fn build_stop_request(kind: PolarPmdStreamKind) -> Vec<u8> {
    vec![PMD_OPCODE_STOP_STREAM, kind.measurement_type()]
}

/// Build a PMD "start stream" control-point command.
pub fn build_start_request(kind: PolarPmdStreamKind, settings: PolarPmdStreamSettings) -> Vec<u8> {
    let mut request = vec![PMD_OPCODE_START_STREAM, kind.measurement_type()];
    match kind {
        PolarPmdStreamKind::Acc => {
            if let Some(range_g) = settings.range_g {
                push_u16_setting(&mut request, PMD_SETTING_TYPE_RANGE, range_g);
            }
            push_u16_setting(
                &mut request,
                PMD_SETTING_TYPE_SAMPLE_RATE,
                settings.sample_rate_hz,
            );
            push_u16_setting(
                &mut request,
                PMD_SETTING_TYPE_RESOLUTION,
                settings.resolution_bits,
            );
        }
        PolarPmdStreamKind::Ecg => {
            push_u16_setting(
                &mut request,
                PMD_SETTING_TYPE_SAMPLE_RATE,
                settings.sample_rate_hz,
            );
            push_u16_setting(
                &mut request,
                PMD_SETTING_TYPE_RESOLUTION,
                settings.resolution_bits,
            );
            if let Some(range_g) = settings.range_g {
                push_u16_setting(&mut request, PMD_SETTING_TYPE_RANGE, range_g);
            }
        }
    }
    if let Some(channels) = settings.channels {
        request.extend_from_slice(&[PMD_SETTING_TYPE_CHANNELS, 0x01, channels]);
    }
    request
}

/// LSL descriptor and channel schema for a Polar HR/RR outlet.
pub fn polar_heart_rate_lsl_stream(
    name: impl Into<String>,
) -> (LslStreamDescriptor, LslChannelSchema) {
    (
        LslStreamDescriptor::new(
            name,
            POLAR_HEART_RATE_STREAM_TYPE,
            2,
            LslChannelFormat::Float32,
        )
        .with_role(LslStreamRole::PolarHeartRate),
        LslChannelSchema::new(
            vec!["bpm".to_string(), "last_rr_ms".to_string()],
            Some("bpm/ms".to_string()),
        ),
    )
}

/// LSL descriptor and channel schema for a Polar ECG outlet.
pub fn polar_ecg_lsl_stream(name: impl Into<String>) -> (LslStreamDescriptor, LslChannelSchema) {
    (
        LslStreamDescriptor::new(name, POLAR_ECG_STREAM_TYPE, 1, LslChannelFormat::Float32)
            .with_nominal_srate_hz(130.0)
            .with_role(LslStreamRole::PolarEcg),
        LslChannelSchema::new(vec!["microvolts".to_string()], Some("uV".to_string())),
    )
}

/// LSL descriptor and channel schema for a Polar accelerometer outlet.
pub fn polar_acc_lsl_stream(name: impl Into<String>) -> (LslStreamDescriptor, LslChannelSchema) {
    (
        LslStreamDescriptor::new(name, POLAR_ACC_STREAM_TYPE, 3, LslChannelFormat::Float32)
            .with_nominal_srate_hz(200.0)
            .with_role(LslStreamRole::PolarAccelerometer),
        LslChannelSchema::new(
            vec!["x_mg".to_string(), "y_mg".to_string(), "z_mg".to_string()],
            Some("mg".to_string()),
        ),
    )
}

fn decode_sensor_contact(flags: u8) -> HeartRateSensorContact {
    match (flags >> 1) & 0b11 {
        0b00 => HeartRateSensorContact::NotSupported,
        0b10 => HeartRateSensorContact::SupportedNotDetected,
        0b11 => HeartRateSensorContact::SupportedDetected,
        _ => HeartRateSensorContact::Unknown,
    }
}

fn decode_rr_intervals(payload: &[u8], mut offset: usize) -> Result<Vec<f32>, PolarDecodeError> {
    let mut intervals = Vec::new();
    while offset + 1 < payload.len() {
        let raw = read_le_u16(payload, offset)?;
        intervals.push(raw as f32 * 1000.0 / 1024.0);
        offset += 2;
    }
    Ok(intervals)
}

fn validate_pmd_header(payload: &[u8], expected_type: u8) -> Result<(), PolarDecodeError> {
    if payload.len() < PMD_HEADER_SIZE {
        return Err(PolarDecodeError::PayloadTooShort {
            expected_at_least: PMD_HEADER_SIZE,
            actual: payload.len(),
        });
    }
    if payload[0] != expected_type {
        return Err(PolarDecodeError::UnexpectedMeasurementType {
            expected: expected_type,
            actual: payload[0],
        });
    }
    Ok(())
}

fn read_pmd_timestamp_ns(payload: &[u8]) -> Result<u64, PolarDecodeError> {
    if payload.len() < 9 {
        return Err(PolarDecodeError::PayloadTooShort {
            expected_at_least: 9,
            actual: payload.len(),
        });
    }
    Ok(u64::from_le_bytes([
        payload[1], payload[2], payload[3], payload[4], payload[5], payload[6], payload[7],
        payload[8],
    ]))
}

fn read_le_u16(payload: &[u8], offset: usize) -> Result<u16, PolarDecodeError> {
    if offset + 1 >= payload.len() {
        return Err(PolarDecodeError::PayloadTooShort {
            expected_at_least: offset + 2,
            actual: payload.len(),
        });
    }
    Ok(u16::from_le_bytes([payload[offset], payload[offset + 1]]))
}

fn read_i24_le(chunk: &[u8]) -> i32 {
    let raw = (chunk[0] as i32) | ((chunk[1] as i32) << 8) | ((chunk[2] as i32) << 16);
    if (raw & 0x0080_0000) != 0 {
        raw | !0x00ff_ffff
    } else {
        raw
    }
}

fn push_u16_setting(request: &mut Vec<u8>, setting_type: u8, value: u16) {
    request.extend_from_slice(&[
        setting_type,
        0x01,
        (value & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
    ]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn default_gatt_paths_are_valid() {
        let paths = PolarGattPaths::default();

        assert!(paths.heart_rate_measurement.is_valid());
        assert!(paths.pmd_control_point.is_valid());
        assert!(paths.pmd_data.is_valid());
    }

    #[test]
    fn decodes_heart_rate_rr_payload() {
        let payload = [0x10, 60, 0x20, 0x03, 0x00, 0x04];
        let reading = decode_heart_rate_measurement(&payload).unwrap();

        assert_eq!(reading.bpm, 60);
        assert_eq!(reading.energy_expended, None);
        assert_eq!(reading.rr_intervals_ms, vec![781.25, 1000.0]);
        assert!(reading.has_rr_intervals());
    }

    #[test]
    fn decodes_heart_rate_16_bit_and_energy_payload() {
        let payload = [0x09, 0x2c, 0x01, 0x64, 0x00];
        let reading = decode_heart_rate_measurement(&payload).unwrap();

        assert_eq!(reading.bpm, 300);
        assert_eq!(reading.energy_expended, Some(100));
        assert!(reading.rr_intervals_ms.is_empty());
    }

    #[test]
    fn decodes_ecg_frame() {
        let payload = [
            PMD_MEASUREMENT_TYPE_ECG,
            1,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0x00,
            0x01,
            0x00,
            0x00,
            0xff,
            0xff,
            0xff,
        ];
        let frame = decode_ecg_pmd_frame(&payload).unwrap();

        assert_eq!(frame.sensor_timestamp_ns, 1);
        assert_eq!(frame.samples_microvolts, vec![1, -1]);
    }

    #[test]
    fn decodes_uncompressed_acc_frame() {
        let payload = [
            PMD_MEASUREMENT_TYPE_ACC,
            2,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0x01,
            0xe8,
            0x03,
            0x18,
            0xfc,
            0x00,
            0x00,
        ];
        let frame = decode_uncompressed_acc_pmd_frame(&payload).unwrap();

        assert_eq!(frame.sensor_timestamp_ns, 2);
        assert_eq!(
            frame.samples_mg,
            vec![PolarAccSample {
                x_mg: 1000,
                y_mg: -1000,
                z_mg: 0
            }]
        );
        assert_eq!(frame.samples_mg[0].to_g(), [1.0, -1.0, 0.0]);
    }

    #[test]
    fn builds_pmd_commands() {
        assert_eq!(
            build_get_settings_request(PolarPmdStreamKind::Ecg),
            vec![0x01, PMD_MEASUREMENT_TYPE_ECG]
        );
        assert_eq!(
            build_stop_request(PolarPmdStreamKind::Acc),
            vec![0x03, PMD_MEASUREMENT_TYPE_ACC]
        );
        assert_eq!(
            build_start_request(
                PolarPmdStreamKind::Acc,
                PolarPmdStreamSettings::h10_acc_default()
            ),
            vec![
                0x02,
                PMD_MEASUREMENT_TYPE_ACC,
                0x02,
                0x01,
                0x08,
                0x00,
                0x00,
                0x01,
                0xc8,
                0x00,
                0x01,
                0x01,
                0x10,
                0x00
            ]
        );
    }

    #[test]
    fn creates_lsl_stream_descriptors() {
        let (descriptor, schema) = polar_acc_lsl_stream("polar_acc");

        assert!(descriptor.is_valid());
        assert_eq!(descriptor.role, Some(LslStreamRole::PolarAccelerometer));
        assert_eq!(schema.labels, vec!["x_mg", "y_mg", "z_mg"]);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn polar_acc_frame_round_trips_with_serde() {
        let frame = PolarAccFrame {
            sensor_timestamp_ns: 2,
            samples_mg: vec![PolarAccSample {
                x_mg: 1000,
                y_mg: -1000,
                z_mg: 0,
            }],
        };

        let encoded = serde_json::to_string(&frame).expect("frame should serialize");
        let decoded: PolarAccFrame =
            serde_json::from_str(&encoded).expect("frame should deserialize");

        assert_eq!(decoded, frame);
    }
}
