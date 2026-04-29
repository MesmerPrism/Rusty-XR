//! Framework-neutral BLE and Android Bluetooth contracts for Rusty XR.
//!
//! This crate intentionally contains pure data models only. Android permission
//! prompts, GATT handles, Bluetooth adapters, and background services belong in
//! the app shell or an optional adapter crate.
//!
//! Enable the `serde` feature to serialize public scan, GATT, and permission
//! models for operator tools or schemas.

/// Crate version exposed for lightweight smoke checks.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Bluetooth SIG base UUID used when expanding 16-bit assigned numbers.
pub const BLUETOOTH_SIG_BASE_SUFFIX: &str = "-0000-1000-8000-00805f9b34fb";

/// Client Characteristic Configuration Descriptor UUID.
pub const CCCD_DESCRIPTOR_UUID: &str = "00002902-0000-1000-8000-00805f9b34fb";

/// Android manifest permission for BLE scanning on Android 12/API 31+.
pub const ANDROID_PERMISSION_BLUETOOTH_SCAN: &str = "android.permission.BLUETOOTH_SCAN";

/// Android manifest permission for BLE connections on Android 12/API 31+.
pub const ANDROID_PERMISSION_BLUETOOTH_CONNECT: &str = "android.permission.BLUETOOTH_CONNECT";

/// Android manifest permission for BLE advertising on Android 12/API 31+.
pub const ANDROID_PERMISSION_BLUETOOTH_ADVERTISE: &str = "android.permission.BLUETOOTH_ADVERTISE";

/// Legacy Bluetooth permission for Android 11/API 30 and lower.
pub const ANDROID_PERMISSION_BLUETOOTH: &str = "android.permission.BLUETOOTH";

/// Legacy Bluetooth administration permission for Android 11/API 30 and lower.
pub const ANDROID_PERMISSION_BLUETOOTH_ADMIN: &str = "android.permission.BLUETOOTH_ADMIN";

/// Runtime location permission needed for BLE scans on Android 11/API 30 and lower.
pub const ANDROID_PERMISSION_ACCESS_FINE_LOCATION: &str = "android.permission.ACCESS_FINE_LOCATION";

/// A normalized BLE UUID string.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BleUuid(String);

impl BleUuid {
    /// Creates a normalized lowercase UUID.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_ascii_lowercase())
    }

    /// Expands a 16-bit Bluetooth SIG assigned number into a 128-bit UUID.
    pub fn from_u16(assigned_number: u16) -> Self {
        Self(format!(
            "0000{assigned_number:04x}{BLUETOOTH_SIG_BASE_SUFFIX}"
        ))
    }

    /// Returns the normalized UUID string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Performs a lightweight UUID shape check.
    pub fn is_canonical_128(&self) -> bool {
        let value = self.0.as_bytes();
        value.len() == 36
            && [8, 13, 18, 23].iter().all(|index| value[*index] == b'-')
            && value
                .iter()
                .enumerate()
                .all(|(index, byte)| [8, 13, 18, 23].contains(&index) || byte.is_ascii_hexdigit())
    }
}

impl From<&str> for BleUuid {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable device identity as seen by a platform BLE adapter.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BleDeviceIdentity {
    pub name: Option<String>,
    pub address: Option<String>,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
}

impl BleDeviceIdentity {
    /// Returns true when the identity has enough information for diagnostics.
    pub fn is_identified(&self) -> bool {
        self.name
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
            || self
                .address
                .as_deref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
    }
}

/// A BLE scan result snapshot.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct BleScanResult {
    pub device: BleDeviceIdentity,
    pub rssi_dbm: Option<i16>,
    pub connectable: bool,
    pub seen_time_ns: u64,
}

impl BleScanResult {
    /// Creates a scan result with an explicit monotonic timestamp.
    pub fn new(device: BleDeviceIdentity, seen_time_ns: u64) -> Self {
        Self {
            device,
            rssi_dbm: None,
            connectable: true,
            seen_time_ns,
        }
    }

    /// Returns whether the device name starts with a prefix.
    pub fn name_has_prefix(&self, prefix: &str) -> bool {
        self.device
            .name
            .as_deref()
            .map(|name| name.starts_with(prefix))
            .unwrap_or(false)
    }
}

/// Platform-neutral BLE connection state.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BleConnectionState {
    Idle,
    Scanning,
    Connecting,
    DiscoveringServices,
    Connected,
    Disconnecting,
    Disconnected,
    Fault,
}

/// A specific GATT characteristic under a service UUID.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GattCharacteristicPath {
    pub service_uuid: BleUuid,
    pub characteristic_uuid: BleUuid,
}

impl GattCharacteristicPath {
    /// Creates a service/characteristic path.
    pub fn new(service_uuid: impl Into<BleUuid>, characteristic_uuid: impl Into<BleUuid>) -> Self {
        Self {
            service_uuid: service_uuid.into(),
            characteristic_uuid: characteristic_uuid.into(),
        }
    }

    /// Returns true if both UUIDs are canonical 128-bit values.
    pub fn is_valid(&self) -> bool {
        self.service_uuid.is_canonical_128() && self.characteristic_uuid.is_canonical_128()
    }
}

/// Desired notification mode for a GATT characteristic CCCD.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GattNotificationMode {
    Notify,
    Indicate,
}

impl GattNotificationMode {
    /// CCCD little-endian value used to enable the mode.
    pub const fn cccd_enable_value(self) -> [u8; 2] {
        match self {
            Self::Notify => [0x01, 0x00],
            Self::Indicate => [0x02, 0x00],
        }
    }
}

/// Description of a GATT operation an adapter should perform.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GattOperation {
    DiscoverService(BleUuid),
    EnableNotifications {
        characteristic: GattCharacteristicPath,
        mode: GattNotificationMode,
    },
    DisableNotifications(GattCharacteristicPath),
    Write {
        characteristic: GattCharacteristicPath,
        payload: Vec<u8>,
        response: GattWriteResponse,
    },
    Read(GattCharacteristicPath),
}

/// Whether a GATT write expects a platform response.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GattWriteResponse {
    WithResponse,
    WithoutResponse,
}

/// Android BLE permissions needed for a scan/connect workflow.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidBlePermissionSet {
    pub target_api_level: u32,
    pub manifest_permissions: Vec<String>,
    pub runtime_permissions: Vec<String>,
    pub scan_never_for_location: bool,
}

impl AndroidBlePermissionSet {
    /// Returns a conservative permission plan for scanning and connecting.
    ///
    /// Android 12/API 31+ uses Nearby Devices runtime permissions. Android 11/API
    /// 30 and lower requires location for BLE scans.
    pub fn scan_and_connect(target_api_level: u32, scan_derives_location: bool) -> Self {
        if target_api_level >= 31 {
            let mut manifest_permissions = permission_list(&[
                ANDROID_PERMISSION_BLUETOOTH_SCAN,
                ANDROID_PERMISSION_BLUETOOTH_CONNECT,
                ANDROID_PERMISSION_BLUETOOTH,
                ANDROID_PERMISSION_BLUETOOTH_ADMIN,
            ]);
            let mut runtime_permissions = permission_list(&[
                ANDROID_PERMISSION_BLUETOOTH_SCAN,
                ANDROID_PERMISSION_BLUETOOTH_CONNECT,
            ]);
            if scan_derives_location {
                manifest_permissions.push(ANDROID_PERMISSION_ACCESS_FINE_LOCATION.to_string());
                runtime_permissions.push(ANDROID_PERMISSION_ACCESS_FINE_LOCATION.to_string());
            }
            Self {
                target_api_level,
                manifest_permissions,
                runtime_permissions,
                scan_never_for_location: !scan_derives_location,
            }
        } else {
            Self {
                target_api_level,
                manifest_permissions: permission_list(&[
                    ANDROID_PERMISSION_BLUETOOTH,
                    ANDROID_PERMISSION_BLUETOOTH_ADMIN,
                    ANDROID_PERMISSION_ACCESS_FINE_LOCATION,
                ]),
                runtime_permissions: permission_list(&[ANDROID_PERMISSION_ACCESS_FINE_LOCATION]),
                scan_never_for_location: false,
            }
        }
    }
}

fn permission_list(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_workspace_version() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn expands_assigned_numbers() {
        assert_eq!(
            BleUuid::from_u16(0x180d).as_str(),
            "0000180d-0000-1000-8000-00805f9b34fb"
        );
        assert!(BleUuid::from_u16(0x2a37).is_canonical_128());
    }

    #[test]
    fn validates_characteristic_paths() {
        let path =
            GattCharacteristicPath::new(BleUuid::from_u16(0x180d), BleUuid::from_u16(0x2a37));

        assert!(path.is_valid());
    }

    #[test]
    fn exposes_cccd_values() {
        assert_eq!(
            GattNotificationMode::Notify.cccd_enable_value(),
            [0x01, 0x00]
        );
        assert_eq!(
            GattNotificationMode::Indicate.cccd_enable_value(),
            [0x02, 0x00]
        );
    }

    #[test]
    fn android_12_scan_permissions_are_runtime_nearby_devices() {
        let permissions = AndroidBlePermissionSet::scan_and_connect(35, false);

        assert!(permissions
            .runtime_permissions
            .iter()
            .any(|permission| permission == ANDROID_PERMISSION_BLUETOOTH_SCAN));
        assert!(permissions
            .runtime_permissions
            .iter()
            .any(|permission| permission == ANDROID_PERMISSION_BLUETOOTH_CONNECT));
        assert!(!permissions
            .runtime_permissions
            .iter()
            .any(|permission| permission == ANDROID_PERMISSION_ACCESS_FINE_LOCATION));
        assert!(permissions.scan_never_for_location);
    }

    #[test]
    fn legacy_scan_permissions_include_runtime_location() {
        let permissions = AndroidBlePermissionSet::scan_and_connect(30, false);

        assert_eq!(
            permissions.runtime_permissions,
            vec![ANDROID_PERMISSION_ACCESS_FINE_LOCATION.to_string()]
        );
        assert!(!permissions.scan_never_for_location);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn gatt_operation_round_trips_with_serde() {
        let operation = GattOperation::EnableNotifications {
            characteristic: GattCharacteristicPath::new(
                BleUuid::from_u16(0x180d),
                BleUuid::from_u16(0x2a37),
            ),
            mode: GattNotificationMode::Notify,
        };

        let encoded = serde_json::to_string(&operation).expect("operation should serialize");
        let decoded: GattOperation =
            serde_json::from_str(&encoded).expect("operation should deserialize");

        assert_eq!(decoded, operation);
    }
}
