//! Device information: bundled sample device and random generator.

use serde::{Deserialize, Serialize};

/// Device info (the `register_dev` request body, field names match the `KuGou`
/// protocol exactly, including the sensor `xxxValue` naming).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    /// Available RAM in bytes.
    pub available_ram_size: u64,
    /// Available internal storage in bytes.
    pub available_rom_size: u64,
    /// Available external storage in bytes.
    pub available_sd_size: u64,
    /// Baseband version.
    pub baseband_ver: String,
    /// Battery percentage 0-100.
    pub battery_level: u32,
    /// Battery status (3 = normal).
    pub battery_status: u32,
    /// Brand.
    pub brand: String,
    /// Build serial.
    pub build_serial: String,
    /// Device codename.
    pub device: String,
    /// IMEI.
    pub imei: String,
    /// SIM serial.
    pub imsi: String,
    /// Manufacturer.
    pub manufacturer: String,
    /// Device UUID.
    pub uuid: String,
    /// Has accelerometer.
    pub accelerometer: bool,
    /// Accelerometer value.
    #[serde(rename = "accelerometerValue")]
    pub accelerometer_value: String,
    /// Has gravity sensor.
    pub gravity: bool,
    /// Gravity sensor value.
    #[serde(rename = "gravityValue")]
    pub gravity_value: String,
    /// Has gyroscope.
    pub gyroscope: bool,
    /// Gyroscope value.
    #[serde(rename = "gyroscopeValue")]
    pub gyroscope_value: String,
    /// Has light sensor.
    pub light: bool,
    /// Light sensor value.
    #[serde(rename = "lightValue")]
    pub light_value: String,
    /// Has magnetic sensor.
    pub magnetic: bool,
    /// Magnetic sensor value.
    #[serde(rename = "magneticValue")]
    pub magnetic_value: String,
    /// Has orientation sensor.
    pub orientation: bool,
    /// Orientation sensor value.
    #[serde(rename = "orientationValue")]
    pub orientation_value: String,
    /// Has pressure sensor.
    pub pressure: bool,
    /// Pressure sensor value.
    #[serde(rename = "pressureValue")]
    pub pressure_value: String,
    /// Has step counter (wire name keeps the snake form).
    #[serde(rename = "step_counter")]
    pub step_counter: bool,
    /// Step counter value.
    #[serde(rename = "step_counterValue")]
    pub step_counter_value: String,
    /// Has temperature sensor.
    pub temperature: bool,
    /// Temperature sensor value.
    #[serde(rename = "temperatureValue")]
    pub temperature_value: String,
}

impl Device {
    /// A random device: fresh imei / uuid / buildSerial / battery / storage,
    /// everything else inherited from the bundled model. Fields can be
    /// overridden afterwards through struct update.
    #[must_use]
    pub fn random() -> Self {
        random_device()
    }
}

impl Default for Device {
    fn default() -> Self {
        DEFAULT_DEVICE.clone()
    }
}

/// Derive a MID from a dfid: `md5(dfid)`.
///
/// Verified live: with the default dfid (`"-"`), `mid = md5("-")` passes the
/// trackercdn key check; after registration the server-provided mid wins.
#[must_use]
pub fn derive_mid(dfid: &str) -> String {
    crate::sign::md5(dfid)
}

/// KuGouMusicApi-ecosystem MID form: interpret the md5 hex digest as a 128-bit
/// big integer and render it as a decimal string (38-39 digits).
///
/// Use this to stay compatible with existing cookie-based deployments
/// (`KUGOU_API_MID`).
#[must_use]
pub fn calculate_mid(input: &str) -> String {
    let digest = crate::sign::md5(input);
    u128::from_str_radix(&digest, 16)
        .unwrap_or_default()
        .to_string()
}

/// Lazily constructed bundled sample device.
///
/// Field order matters: the `register_dev` body is `JSON.stringify(device)` and
/// this struct mirrors the TS interface declaration order.
pub static DEFAULT_DEVICE: std::sync::LazyLock<Device> = std::sync::LazyLock::new(|| Device {
    available_ram_size: 4_983_533_568,
    available_rom_size: 48_114_719,
    available_sd_size: 48_114_717,
    baseband_ver: String::new(),
    battery_level: 100,
    battery_status: 3,
    brand: "Redmi".to_owned(),
    build_serial: "unknown".to_owned(),
    device: "marble".to_owned(),
    imei: "36ec3614ad302c846b72bed309aeb598".to_owned(),
    imsi: String::new(),
    manufacturer: "Xiaomi".to_owned(),
    uuid: "36ec3614ad302c846b72bed309aeb598".to_owned(),
    accelerometer: false,
    accelerometer_value: String::new(),
    gravity: false,
    gravity_value: String::new(),
    gyroscope: false,
    gyroscope_value: String::new(),
    light: false,
    light_value: String::new(),
    magnetic: false,
    magnetic_value: String::new(),
    orientation: false,
    orientation_value: String::new(),
    pressure: false,
    pressure_value: String::new(),
    step_counter: false,
    step_counter_value: String::new(),
    temperature: false,
    temperature_value: String::new(),
});

const RAM_CHOICES: [u64; 7] = [2, 3, 4, 6, 8, 12, 16];

/// Generate a random device: random imei / uuid / buildSerial / battery /
/// storage, other fields inherited from the bundled model.
#[must_use]
pub fn random_device() -> Device {
    use rand::RngExt;
    let mut rng = rand::rng();

    let id = uuid::Uuid::new_v4().simple().to_string();
    let ram = RAM_CHOICES[rng.random_range(0..RAM_CHOICES.len())] * 1024u64.pow(3);
    let rom = 32 * 1024u64.pow(2) + u64::from(rng.random_range(0..4096u32)) * 1024u64.pow(2);

    Device {
        imei: id.clone(),
        uuid: id,
        build_serial: uuid::Uuid::new_v4().simple().to_string()[..10].to_owned(),
        available_ram_size: ram,
        available_rom_size: rom,
        available_sd_size: rom,
        battery_level: 20 + rng.random_range(0..81),
        ..DEFAULT_DEVICE.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_device_matches_reference() {
        assert_eq!(DEFAULT_DEVICE.brand, "Redmi");
        assert_eq!(DEFAULT_DEVICE.device, "marble");
        assert_eq!(DEFAULT_DEVICE.manufacturer, "Xiaomi");
        assert_eq!(DEFAULT_DEVICE.imei, "36ec3614ad302c846b72bed309aeb598");
        assert_eq!(DEFAULT_DEVICE.available_ram_size, 4_983_533_568);
        assert!(!DEFAULT_DEVICE.accelerometer);
    }

    #[test]
    fn default_device_serialization_field_order() {
        // The register_dev body is JSON.stringify(device) — field order must
        // match the TS interface exactly.
        let json = serde_json::to_string(&*DEFAULT_DEVICE).unwrap();
        let expected_prefix = "{\"availableRamSize\":4983533568,\"availableRomSize\":48114719";
        assert!(
            json.starts_with(expected_prefix),
            "unexpected prefix: {json}"
        );
        assert!(json.contains("\"step_counterValue\":\"\""));
        assert!(json.ends_with("\"temperatureValue\":\"\"}"));
    }

    #[test]
    fn random_device_overrides() {
        let a = random_device();
        let b = random_device();
        assert_ne!(a.imei, b.imei);
        assert_eq!(a.brand, "Redmi");
        assert!((20..=100).contains(&a.battery_level));
        assert_eq!(a.imei.len(), 32);
        assert_eq!(a.build_serial.len(), 10);
    }

    #[test]
    fn mid_derivation() {
        assert_eq!(derive_mid("-"), "336d5ebc5436534e61d16e63ddfca327");
        // md5('-') interpreted as a 128-bit integer
        assert_eq!(calculate_mid("-"), "68358509610070717889884130747296293671");
    }
}
