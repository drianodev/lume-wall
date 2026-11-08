use chrono::{DateTime, Datelike, Timelike, Utc};

/// Coordenadas geográficas do observador
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SolarCoordinates {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub elevation: f64,
}

impl Default for SolarCoordinates {
    fn default() -> Self {
        Self {
            latitude: -23.5505,
            longitude: -46.6333,
            elevation: 760.0,
        }
    }
}

/// Posição aparente do Sol
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolarPosition {
    /// Azimute em graus (0° Norte, 90° Leste, 180° Sul, 270° Oeste)
    pub azimuth: f64,
    /// Altitude/Elevação em graus (-90° Nadir, 0° Horizonte, 90° Zênite)
    pub altitude: f64,
}

/// Calcula a posição solar aparente usando uma aproximação de alta precisão
/// do algoritmo NOAA Solar Calculator (offline, sem dependência de rede)
pub fn calculate_solar_position(coords: &SolarCoordinates, time: DateTime<Utc>) -> SolarPosition {
    let lat = coords.latitude;
    let lon = coords.longitude;

    // Dia juliano fracionário
    let year = time.year() as f64;
    let month = time.month() as f64;
    let day = time.day() as f64;
    let hour = time.hour() as f64 + (time.minute() as f64 / 60.0) + (time.second() as f64 / 3600.0);

    let (y, m) = if month <= 2.0 {
        (year - 1.0, month + 12.0)
    } else {
        (year, month)
    };

    let a = (y / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    let jd = (365.25 * (y + 4716.0)).floor() + (30.6001 * (m + 1.0)).floor() + day + b - 1524.5
        + (hour / 24.0);

    // Século juliano a partir de J2000.0
    let t = (jd - 2451545.0) / 36525.0;

    // Longitude geométrica média do sol (graus)
    let mut l0 = 280.46646 + t * (36000.76983 + 0.0003032 * t);
    l0 = l0.rem_euclid(360.0);

    // Anomalia média do sol (graus)
    let m_deg = 357.52911 + t * (35999.05029 - 0.0001537 * t);
    let m_rad = m_deg.to_radians();

    // Equação do centro
    let c = (1.914602 - t * (0.004817 + 0.000014 * t)) * m_rad.sin()
        + (0.019993 - 0.000101 * t) * (2.0 * m_rad).sin()
        + 0.000289 * (3.0 * m_rad).sin();

    // Longitude verdadeira do sol
    let true_lon = l0 + c;

    // Obliquidade da eclíptica
    let eps0 = 23.0 + (26.0 / 60.0) + (21.448 - t * (46.8150 + t * (0.00059 - t * 0.001813))) / 3600.0;
    let eps = eps0 + 0.00256 * ((125.04 - 1934.136 * t).to_radians()).cos();

    // Declinação e ascensão reta do sol
    let true_lon_rad = true_lon.to_radians();
    let eps_rad = eps.to_radians();

    let sin_dec = eps_rad.sin() * true_lon_rad.sin();
    let dec = sin_dec.asin(); // radianos

    // Equação do tempo (minutos)
    let var_y = (eps_rad / 2.0).tan().powi(2);
    let l0_rad = l0.to_radians();
    let eq_time = 4.0
        * (var_y * (2.0 * l0_rad).sin() - 2.0 * 0.016708634 * m_rad.sin()
            + 4.0 * 0.016708634 * var_y * m_rad.sin() * (2.0 * l0_rad).cos()
            - 0.5 * var_y.powi(2) * (4.0 * l0_rad).sin()
            - 1.25 * (0.016708634_f64).powi(2) * (2.0 * m_rad).sin())
        .to_degrees();

    // Ângulo horário solar verdadeiro
    let time_offset = eq_time + 4.0 * lon;
    let true_solar_time = (hour * 60.0 + time_offset).rem_euclid(1440.0);
    let mut ha_deg = (true_solar_time / 4.0) - 180.0;
    if ha_deg < -180.0 {
        ha_deg += 360.0;
    }
    let ha_rad = ha_deg.to_radians();

    // Ângulo de zênite solar
    let lat_rad = lat.to_radians();
    let cos_zenith = lat_rad.sin() * dec.sin() + lat_rad.cos() * dec.cos() * ha_rad.cos();
    let cos_zenith_clamped = cos_zenith.clamp(-1.0, 1.0);
    let zenith_rad = cos_zenith_clamped.acos();

    // Altitude (elevação) sem refração
    let elevation = 90.0 - zenith_rad.to_degrees();

    // Azimute
    let cos_az = (lat_rad.sin() * cos_zenith_clamped - dec.sin())
        / (lat_rad.cos() * zenith_rad.sin());
    let cos_az_clamped = cos_az.clamp(-1.0, 1.0);
    let mut azimuth = 180.0 - cos_az_clamped.acos().to_degrees();
    if ha_deg > 0.0 {
        azimuth = 360.0 - azimuth;
    }

    SolarPosition {
        azimuth: (azimuth + 180.0).rem_euclid(360.0), // Normalizado 0..360
        altitude: elevation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_solar_position_sao_paulo_noon() {
        let coords = SolarCoordinates::default(); // São Paulo
        // 2026-11-08 15:00 UTC (~12:00 BRT solar)
        let dt = Utc.with_ymd_and_hms(2026, 11, 8, 15, 0, 0).unwrap();
        let pos = calculate_solar_position(&coords, dt);
        // Ao meio dia em novembro em SP, o sol está alto no céu (altitude > 60°)
        assert!(pos.altitude > 60.0, "Altitude esperada > 60°, deu {}", pos.altitude);
    }
}
