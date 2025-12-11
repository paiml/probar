//! Geolocation Mocking (Feature 16)
//!
//! Mock GPS coordinates and location data for testing location-based features.
//!
//! ## EXTREME TDD: Tests written FIRST per spec
//!
//! ## Toyota Way Application:
//! - **Poka-Yoke**: Type-safe coordinates prevent invalid lat/long values
//! - **Muda**: Efficient location simulation without real GPS overhead

#![allow(clippy::unreadable_literal)]

use std::collections::HashMap;

/// Geographic position with coordinates and accuracy
#[derive(Debug, Clone, PartialEq)]
pub struct GeolocationPosition {
    /// Latitude in decimal degrees (-90.0 to 90.0)
    pub latitude: f64,
    /// Longitude in decimal degrees (-180.0 to 180.0)
    pub longitude: f64,
    /// Accuracy in meters
    pub accuracy: f64,
    /// Altitude in meters (optional)
    pub altitude: Option<f64>,
    /// Altitude accuracy in meters (optional)
    pub altitude_accuracy: Option<f64>,
    /// Heading in degrees (0-360, optional)
    pub heading: Option<f64>,
    /// Speed in meters per second (optional)
    pub speed: Option<f64>,
}

impl GeolocationPosition {
    /// Create a new position with basic coordinates
    ///
    /// # Arguments
    /// * `latitude` - Latitude in decimal degrees (-90.0 to 90.0)
    /// * `longitude` - Longitude in decimal degrees (-180.0 to 180.0)
    /// * `accuracy` - Accuracy in meters
    ///
    /// # Panics
    /// Panics if latitude or longitude are out of valid range
    #[must_use]
    pub fn new(latitude: f64, longitude: f64, accuracy: f64) -> Self {
        assert!(
            (-90.0..=90.0).contains(&latitude),
            "Latitude must be between -90 and 90 degrees"
        );
        assert!(
            (-180.0..=180.0).contains(&longitude),
            "Longitude must be between -180 and 180 degrees"
        );
        assert!(accuracy >= 0.0, "Accuracy must be non-negative");

        Self {
            latitude,
            longitude,
            accuracy,
            altitude: None,
            altitude_accuracy: None,
            heading: None,
            speed: None,
        }
    }

    /// Set altitude
    #[must_use]
    pub fn with_altitude(mut self, altitude: f64, accuracy: f64) -> Self {
        self.altitude = Some(altitude);
        self.altitude_accuracy = Some(accuracy);
        self
    }

    /// Set heading (direction of travel)
    #[must_use]
    pub fn with_heading(mut self, heading: f64) -> Self {
        assert!(
            (0.0..=360.0).contains(&heading),
            "Heading must be between 0 and 360 degrees"
        );
        self.heading = Some(heading);
        self
    }

    /// Set speed
    #[must_use]
    pub fn with_speed(mut self, speed: f64) -> Self {
        assert!(speed >= 0.0, "Speed must be non-negative");
        self.speed = Some(speed);
        self
    }

    // === Preset Locations ===

    /// New York City, USA (Times Square)
    #[must_use]
    pub fn new_york() -> Self {
        Self::new(40.758896, -73.985130, 10.0)
    }

    /// Tokyo, Japan (Shibuya Crossing)
    #[must_use]
    pub fn tokyo() -> Self {
        Self::new(35.659492, 139.700472, 10.0)
    }

    /// London, UK (Trafalgar Square)
    #[must_use]
    pub fn london() -> Self {
        Self::new(51.508039, -0.128069, 10.0)
    }

    /// Paris, France (Eiffel Tower)
    #[must_use]
    pub fn paris() -> Self {
        Self::new(48.858370, 2.294481, 10.0)
    }

    /// Sydney, Australia (Opera House)
    #[must_use]
    pub fn sydney() -> Self {
        Self::new(-33.856784, 151.215297, 10.0)
    }

    /// San Francisco, USA (Golden Gate Bridge)
    #[must_use]
    pub fn san_francisco() -> Self {
        Self::new(37.820587, -122.478264, 10.0)
    }

    /// Berlin, Germany (Brandenburg Gate)
    #[must_use]
    pub fn berlin() -> Self {
        Self::new(52.516275, 13.377704, 10.0)
    }

    /// Singapore (Marina Bay Sands)
    #[must_use]
    pub fn singapore() -> Self {
        Self::new(1.283404, 103.860435, 10.0)
    }

    /// Dubai, UAE (Burj Khalifa)
    #[must_use]
    pub fn dubai() -> Self {
        Self::new(25.197197, 55.274376, 10.0)
    }

    /// São Paulo, Brazil (Paulista Avenue)
    #[must_use]
    pub fn sao_paulo() -> Self {
        Self::new(-23.561414, -46.655881, 10.0)
    }
}

/// Geolocation mock controller for simulating location changes
#[derive(Debug, Clone)]
pub struct GeolocationMock {
    /// Current mocked position
    current_position: Option<GeolocationPosition>,
    /// Named location presets
    presets: HashMap<String, GeolocationPosition>,
    /// Whether geolocation is enabled
    enabled: bool,
    /// Simulated permission state
    permission_granted: bool,
    /// Error simulation mode
    error_mode: Option<GeolocationError>,
}

/// Simulated geolocation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeolocationError {
    /// Permission denied by user
    PermissionDenied,
    /// Position unavailable
    PositionUnavailable,
    /// Request timed out
    Timeout,
}

impl Default for GeolocationMock {
    fn default() -> Self {
        Self::new()
    }
}

impl GeolocationMock {
    /// Create a new geolocation mock
    #[must_use]
    #[allow(clippy::unused_results)]
    pub fn new() -> Self {
        let mut presets = HashMap::new();

        // Add default presets
        presets.insert("new_york".to_string(), GeolocationPosition::new_york());
        presets.insert("tokyo".to_string(), GeolocationPosition::tokyo());
        presets.insert("london".to_string(), GeolocationPosition::london());
        presets.insert("paris".to_string(), GeolocationPosition::paris());
        presets.insert("sydney".to_string(), GeolocationPosition::sydney());
        presets.insert(
            "san_francisco".to_string(),
            GeolocationPosition::san_francisco(),
        );
        presets.insert("berlin".to_string(), GeolocationPosition::berlin());
        presets.insert("singapore".to_string(), GeolocationPosition::singapore());
        presets.insert("dubai".to_string(), GeolocationPosition::dubai());
        presets.insert("sao_paulo".to_string(), GeolocationPosition::sao_paulo());

        Self {
            current_position: None,
            presets,
            enabled: true,
            permission_granted: true,
            error_mode: None,
        }
    }

    /// Set current position directly
    pub fn set_position(&mut self, position: GeolocationPosition) {
        self.current_position = Some(position);
    }

    /// Set position from preset name
    ///
    /// Returns `true` if preset exists and was set, `false` otherwise
    pub fn set_preset(&mut self, name: &str) -> bool {
        if let Some(position) = self.presets.get(name) {
            self.current_position = Some(position.clone());
            true
        } else {
            false
        }
    }

    /// Add a custom preset location
    pub fn add_preset(&mut self, name: &str, position: GeolocationPosition) {
        let _ = self.presets.insert(name.to_string(), position);
    }

    /// Get current mocked position
    ///
    /// Returns the position or an error based on mock state
    pub fn get_current_position(&self) -> Result<GeolocationPosition, GeolocationError> {
        // Check error mode first
        if let Some(ref error) = self.error_mode {
            return Err(error.clone());
        }

        // Check if geolocation is enabled
        if !self.enabled {
            return Err(GeolocationError::PositionUnavailable);
        }

        // Check permission
        if !self.permission_granted {
            return Err(GeolocationError::PermissionDenied);
        }

        // Return current position or error
        self.current_position
            .clone()
            .ok_or(GeolocationError::PositionUnavailable)
    }

    /// Enable or disable geolocation
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if geolocation is enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Grant or deny geolocation permission
    pub fn set_permission(&mut self, granted: bool) {
        self.permission_granted = granted;
    }

    /// Check if permission is granted
    #[must_use]
    pub fn is_permission_granted(&self) -> bool {
        self.permission_granted
    }

    /// Simulate a specific error
    pub fn simulate_error(&mut self, error: GeolocationError) {
        self.error_mode = Some(error);
    }

    /// Clear error simulation
    pub fn clear_error(&mut self) {
        self.error_mode = None;
    }

    /// List all available presets
    #[must_use]
    pub fn list_presets(&self) -> Vec<&str> {
        self.presets.keys().map(String::as_str).collect()
    }

    /// Get a preset by name
    #[must_use]
    pub fn get_preset(&self, name: &str) -> Option<&GeolocationPosition> {
        self.presets.get(name)
    }

    /// Clear current position
    pub fn clear_position(&mut self) {
        self.current_position = None;
    }

    /// Reset to initial state
    pub fn reset(&mut self) {
        self.current_position = None;
        self.enabled = true;
        self.permission_granted = true;
        self.error_mode = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === GeolocationPosition Tests ===

    #[test]
    fn test_position_new() {
        let pos = GeolocationPosition::new(40.7128, -74.0060, 5.0);
        assert!((pos.latitude - 40.7128).abs() < 0.0001);
        assert!((pos.longitude - (-74.0060)).abs() < 0.0001);
        assert!((pos.accuracy - 5.0).abs() < 0.0001);
        assert!(pos.altitude.is_none());
        assert!(pos.heading.is_none());
        assert!(pos.speed.is_none());
    }

    #[test]
    #[should_panic(expected = "Latitude must be between -90 and 90 degrees")]
    fn test_position_invalid_latitude_high() {
        GeolocationPosition::new(91.0, 0.0, 10.0);
    }

    #[test]
    #[should_panic(expected = "Latitude must be between -90 and 90 degrees")]
    fn test_position_invalid_latitude_low() {
        GeolocationPosition::new(-91.0, 0.0, 10.0);
    }

    #[test]
    #[should_panic(expected = "Longitude must be between -180 and 180 degrees")]
    fn test_position_invalid_longitude_high() {
        GeolocationPosition::new(0.0, 181.0, 10.0);
    }

    #[test]
    #[should_panic(expected = "Longitude must be between -180 and 180 degrees")]
    fn test_position_invalid_longitude_low() {
        GeolocationPosition::new(0.0, -181.0, 10.0);
    }

    #[test]
    #[should_panic(expected = "Accuracy must be non-negative")]
    fn test_position_invalid_accuracy() {
        GeolocationPosition::new(0.0, 0.0, -1.0);
    }

    #[test]
    fn test_position_with_altitude() {
        let pos = GeolocationPosition::new(0.0, 0.0, 10.0).with_altitude(100.0, 5.0);
        assert_eq!(pos.altitude, Some(100.0));
        assert_eq!(pos.altitude_accuracy, Some(5.0));
    }

    #[test]
    fn test_position_with_heading() {
        let pos = GeolocationPosition::new(0.0, 0.0, 10.0).with_heading(90.0);
        assert_eq!(pos.heading, Some(90.0));
    }

    #[test]
    #[should_panic(expected = "Heading must be between 0 and 360 degrees")]
    fn test_position_invalid_heading() {
        GeolocationPosition::new(0.0, 0.0, 10.0).with_heading(361.0);
    }

    #[test]
    fn test_position_with_speed() {
        let pos = GeolocationPosition::new(0.0, 0.0, 10.0).with_speed(10.0);
        assert_eq!(pos.speed, Some(10.0));
    }

    #[test]
    #[should_panic(expected = "Speed must be non-negative")]
    fn test_position_invalid_speed() {
        GeolocationPosition::new(0.0, 0.0, 10.0).with_speed(-1.0);
    }

    #[test]
    fn test_position_builder_chain() {
        let pos = GeolocationPosition::new(40.7128, -74.0060, 5.0)
            .with_altitude(10.0, 2.0)
            .with_heading(45.0)
            .with_speed(5.0);

        assert!((pos.latitude - 40.7128).abs() < 0.0001);
        assert_eq!(pos.altitude, Some(10.0));
        assert_eq!(pos.heading, Some(45.0));
        assert_eq!(pos.speed, Some(5.0));
    }

    // === Preset Location Tests ===

    #[test]
    fn test_preset_new_york() {
        let pos = GeolocationPosition::new_york();
        assert!((pos.latitude - 40.758896).abs() < 0.0001);
        assert!((pos.longitude - (-73.985130)).abs() < 0.0001);
    }

    #[test]
    fn test_preset_tokyo() {
        let pos = GeolocationPosition::tokyo();
        assert!((pos.latitude - 35.659492).abs() < 0.0001);
        assert!((pos.longitude - 139.700472).abs() < 0.0001);
    }

    #[test]
    fn test_preset_london() {
        let pos = GeolocationPosition::london();
        assert!((pos.latitude - 51.508039).abs() < 0.0001);
        assert!((pos.longitude - (-0.128069)).abs() < 0.0001);
    }

    #[test]
    fn test_preset_paris() {
        let pos = GeolocationPosition::paris();
        assert!((pos.latitude - 48.858370).abs() < 0.0001);
        assert!((pos.longitude - 2.294481).abs() < 0.0001);
    }

    #[test]
    fn test_preset_sydney() {
        let pos = GeolocationPosition::sydney();
        assert!(pos.latitude < 0.0); // Southern hemisphere
        assert!(pos.longitude > 0.0);
    }

    #[test]
    fn test_preset_san_francisco() {
        let pos = GeolocationPosition::san_francisco();
        assert!((pos.latitude - 37.820587).abs() < 0.0001);
        assert!((pos.longitude - (-122.478264)).abs() < 0.0001);
    }

    #[test]
    fn test_preset_berlin() {
        let pos = GeolocationPosition::berlin();
        assert!((pos.latitude - 52.516275).abs() < 0.0001);
        assert!((pos.longitude - 13.377704).abs() < 0.0001);
    }

    #[test]
    fn test_preset_singapore() {
        let pos = GeolocationPosition::singapore();
        assert!(pos.latitude > 0.0 && pos.latitude < 2.0); // Near equator
        assert!(pos.longitude > 100.0);
    }

    #[test]
    fn test_preset_dubai() {
        let pos = GeolocationPosition::dubai();
        assert!((pos.latitude - 25.197197).abs() < 0.0001);
        assert!((pos.longitude - 55.274376).abs() < 0.0001);
    }

    #[test]
    fn test_preset_sao_paulo() {
        let pos = GeolocationPosition::sao_paulo();
        assert!(pos.latitude < 0.0); // Southern hemisphere
        assert!(pos.longitude < 0.0); // Western hemisphere
    }

    // === GeolocationMock Tests ===

    #[test]
    fn test_mock_new() {
        let mock = GeolocationMock::new();
        assert!(mock.is_enabled());
        assert!(mock.is_permission_granted());
        assert!(mock.get_current_position().is_err());
    }

    #[test]
    fn test_mock_default() {
        let mock = GeolocationMock::default();
        assert!(mock.is_enabled());
    }

    #[test]
    fn test_mock_set_position() {
        let mut mock = GeolocationMock::new();
        let pos = GeolocationPosition::new(40.0, -74.0, 10.0);
        mock.set_position(pos.clone());

        let result = mock.get_current_position().unwrap();
        assert_eq!(result, pos);
    }

    #[test]
    fn test_mock_set_preset() {
        let mut mock = GeolocationMock::new();
        assert!(mock.set_preset("new_york"));

        let pos = mock.get_current_position().unwrap();
        assert!((pos.latitude - 40.758896).abs() < 0.0001);
    }

    #[test]
    fn test_mock_set_preset_unknown() {
        let mut mock = GeolocationMock::new();
        assert!(!mock.set_preset("unknown_city"));
    }

    #[test]
    fn test_mock_add_custom_preset() {
        let mut mock = GeolocationMock::new();
        let pos = GeolocationPosition::new(12.0, 34.0, 5.0);
        mock.add_preset("custom", pos.clone());

        assert!(mock.set_preset("custom"));
        let result = mock.get_current_position().unwrap();
        assert_eq!(result, pos);
    }

    #[test]
    fn test_mock_list_presets() {
        let mock = GeolocationMock::new();
        let presets = mock.list_presets();
        assert!(presets.contains(&"new_york"));
        assert!(presets.contains(&"tokyo"));
        assert!(presets.contains(&"london"));
        assert!(presets.len() >= 10);
    }

    #[test]
    fn test_mock_get_preset() {
        let mock = GeolocationMock::new();
        let pos = mock.get_preset("tokyo");
        assert!(pos.is_some());

        let unknown = mock.get_preset("unknown");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_mock_permission_denied() {
        let mut mock = GeolocationMock::new();
        mock.set_position(GeolocationPosition::new_york());
        mock.set_permission(false);

        let result = mock.get_current_position();
        assert_eq!(result, Err(GeolocationError::PermissionDenied));
    }

    #[test]
    fn test_mock_disabled() {
        let mut mock = GeolocationMock::new();
        mock.set_position(GeolocationPosition::new_york());
        mock.set_enabled(false);

        let result = mock.get_current_position();
        assert_eq!(result, Err(GeolocationError::PositionUnavailable));
    }

    #[test]
    fn test_mock_simulate_error() {
        let mut mock = GeolocationMock::new();
        mock.set_position(GeolocationPosition::new_york());
        mock.simulate_error(GeolocationError::Timeout);

        let result = mock.get_current_position();
        assert_eq!(result, Err(GeolocationError::Timeout));
    }

    #[test]
    fn test_mock_clear_error() {
        let mut mock = GeolocationMock::new();
        mock.set_position(GeolocationPosition::new_york());
        mock.simulate_error(GeolocationError::Timeout);
        mock.clear_error();

        assert!(mock.get_current_position().is_ok());
    }

    #[test]
    fn test_mock_clear_position() {
        let mut mock = GeolocationMock::new();
        mock.set_position(GeolocationPosition::new_york());
        mock.clear_position();

        assert!(mock.get_current_position().is_err());
    }

    #[test]
    fn test_mock_reset() {
        let mut mock = GeolocationMock::new();
        mock.set_position(GeolocationPosition::new_york());
        mock.set_permission(false);
        mock.set_enabled(false);
        mock.simulate_error(GeolocationError::Timeout);

        mock.reset();

        assert!(mock.is_enabled());
        assert!(mock.is_permission_granted());
        assert!(mock.get_current_position().is_err()); // No position set
    }

    #[test]
    fn test_error_priority() {
        // Error mode takes priority over permission/enabled checks
        let mut mock = GeolocationMock::new();
        mock.set_position(GeolocationPosition::new_york());
        mock.set_permission(true);
        mock.set_enabled(true);
        mock.simulate_error(GeolocationError::Timeout);

        assert_eq!(
            mock.get_current_position(),
            Err(GeolocationError::Timeout)
        );
    }

    #[test]
    fn test_position_equality() {
        let pos1 = GeolocationPosition::new(40.0, -74.0, 10.0);
        let pos2 = GeolocationPosition::new(40.0, -74.0, 10.0);
        let pos3 = GeolocationPosition::new(40.0, -74.0, 15.0);

        assert_eq!(pos1, pos2);
        assert_ne!(pos1, pos3);
    }

    #[test]
    fn test_error_equality() {
        assert_eq!(
            GeolocationError::PermissionDenied,
            GeolocationError::PermissionDenied
        );
        assert_ne!(
            GeolocationError::PermissionDenied,
            GeolocationError::Timeout
        );
    }

    #[test]
    fn test_mock_clone() {
        let mut mock = GeolocationMock::new();
        mock.set_position(GeolocationPosition::new_york());

        let cloned = mock.clone();
        assert!(cloned.get_current_position().is_ok());
    }

    #[test]
    fn test_position_clone() {
        let pos = GeolocationPosition::new_york()
            .with_altitude(100.0, 5.0)
            .with_heading(90.0)
            .with_speed(10.0);

        let cloned = pos.clone();
        assert_eq!(pos, cloned);
    }
}
