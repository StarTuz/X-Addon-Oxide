use anyhow::Result;
use flate2::read::GzDecoder;
use log::{debug, error, info};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use x_adox_bitnet::flight_prompt::WeatherKeyword;

const METAR_URL: &str = "https://aviationweather.gov/data/cache/metars.cache.csv.gz";
const CACHE_TTL_SECS: u64 = 900; // 15 minutes

struct MetarRecord {
    wx_string: Option<String>,
    sky_cover: Option<String>,
    flight_category: Option<String>,
    wind_speed_kt: Option<f32>,
    wind_gust_kt: Option<f32>,
}

pub struct WeatherEngine {
    cache_path: PathBuf,
}

impl Default for WeatherEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherEngine {
    pub fn new() -> Self {
        let config_dir = crate::get_config_root();
        let cache_path = config_dir.join("metars.cache.csv");
        Self { cache_path }
    }

    /// Fetches the live METAR cache from NOAA if the local cache is expired or missing.
    pub fn fetch_live_metars(&self) -> Result<()> {
        let mut download_needed = true;

        if self.cache_path.exists() {
            if let Ok(metadata) = fs::metadata(&self.cache_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = modified.elapsed() {
                        if elapsed.as_secs() < CACHE_TTL_SECS {
                            download_needed = false;
                        }
                    }
                }
            }
        }

        if download_needed {
            info!(
                "METAR cache expired or missing; fetching live data — cache_path={} url={}",
                self.cache_path.display(),
                METAR_URL
            );
            let client = reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()?;

            let response = client.get(METAR_URL).send()?.error_for_status()?;
            let bytes = response.bytes()?;

            debug!("Downloaded gzipped METAR data — compressed_bytes={}", bytes.len());

            // Unzip the GZ stream
            let mut decoder = GzDecoder::new(&bytes[..]);
            let mut csv_data = String::new();
            decoder.read_to_string(&mut csv_data)?;

            // Save to disk cache
            fs::write(&self.cache_path, &csv_data)?;
            info!(
                "METAR cache updated — cache_path={} uncompressed_bytes={}",
                self.cache_path.display(),
                csv_data.len()
            );
        } else {
            debug!("Using valid cached METAR data — cache_path={}", self.cache_path.display());
        }

        Ok(())
    }

    /// Returns the raw METAR string for each requested station ID from the local cache.
    /// Key is uppercase station_id; value is the raw METAR text (e.g. "EGLL 220520Z ...").
    /// Returns an empty map if the cache file is missing or unreadable.
    pub fn get_raw_metars(&self, ids: &[&str]) -> HashMap<String, String> {
        if !self.cache_path.exists() {
            return HashMap::new();
        }

        let targets: std::collections::HashSet<String> =
            ids.iter().map(|s| s.trim().to_uppercase()).collect();

        let mut result = HashMap::new();

        let mut rdr = match csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(false)
            .from_path(&self.cache_path)
        {
            Ok(r) => r,
            Err(_) => return result,
        };

        let mut headers_found = false;
        let mut idx_raw_text = 0usize;
        let mut idx_station = 1usize;

        for record in rdr.records().flatten() {
            if !headers_found {
                if !record.is_empty() && record[0].starts_with("raw_text") {
                    for (i, field) in record.iter().enumerate() {
                        match field {
                            "raw_text" => idx_raw_text = i,
                            "station_id" => idx_station = i,
                            _ => {}
                        }
                    }
                    headers_found = true;
                }
                continue;
            }

            if let Some(station) = record.get(idx_station) {
                let station_up = station.trim().to_uppercase();
                if targets.contains(&station_up) {
                    if let Some(raw) = record.get(idx_raw_text) {
                        let clean = raw.trim().to_string();
                        if !clean.is_empty() {
                            result.insert(station_up, clean);
                        }
                    }
                }
            }
        }

        debug!(
            "get_raw_metars — requested={} found={}",
            targets.len(),
            result.len()
        );
        result
    }

    /// Parses the local METAR cache and maps station IDs to their currently active `WeatherKeyword`.
    pub fn get_global_weather_map(&self) -> Result<HashMap<String, WeatherKeyword>> {
        if !self.cache_path.exists() {
            log::warn!(
                "METAR cache file not found; weather filtering will be skipped — cache_path={}",
                self.cache_path.display()
            );
            return Ok(HashMap::new());
        }

        let mut rdr = csv::ReaderBuilder::new()
            .flexible(true) // NOAA adds 5 lines of metadata comments at the top usually
            .has_headers(false) // We'll handle headers manually to skip comments
            .from_path(&self.cache_path)?;

        let mut map = HashMap::new();
        let mut headers_found = false;

        let mut idx_station = 0;
        let mut idx_wx_string = 0;
        let mut idx_sky_cover = 0;
        let mut idx_flight_cat = 0;
        let mut idx_wind_speed = 0;
        let mut idx_wind_gust = 0;

        for result in rdr.records() {
            let record = match result {
                Ok(r) => r,
                Err(e) => {
                    error!("CSV parsing error: {}", e);
                    continue;
                }
            };

            // Skip NOAA header comment lines & map indexes
            if !headers_found {
                if !record.is_empty() && record[0].starts_with("raw_text") {
                    for (i, field) in record.iter().enumerate() {
                        match field {
                            "station_id" => idx_station = i,
                            "wx_string" => idx_wx_string = i,
                            "sky_cover" if idx_sky_cover == 0 => idx_sky_cover = i,
                            "flight_category" => idx_flight_cat = i,
                            "wind_speed_kt" => idx_wind_speed = i,
                            "wind_gust_kt" => idx_wind_gust = i,
                            _ => {}
                        }
                    }
                    headers_found = true;
                }
                continue;
            }

            // Extract via index
            if record.len() > idx_station {
                let station_id = record[idx_station].to_string();
                if station_id.is_empty() {
                    continue;
                }
                let wx_string = record
                    .get(idx_wx_string)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let sky_cover = record
                    .get(idx_sky_cover)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let flight_category = record
                    .get(idx_flight_cat)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());
                let wind_speed_kt = record
                    .get(idx_wind_speed)
                    .and_then(|s| s.parse::<f32>().ok());
                let wind_gust_kt = record
                    .get(idx_wind_gust)
                    .and_then(|s| s.parse::<f32>().ok());

                let m = MetarRecord {
                    wx_string,
                    sky_cover,
                    flight_category,
                    wind_speed_kt,
                    wind_gust_kt,
                };

                let wx = determine_weather_keyword(&m);
                map.insert(station_id, wx);
            }
        }

        debug!(
            "Loaded global weather map — station_count={} cache_path={}",
            map.len(),
            self.cache_path.display()
        );
        Ok(map)
    }
}

fn determine_weather_keyword(metar: &MetarRecord) -> WeatherKeyword {
    // 1. Analyze explicit phenomena first
    if let Some(wx) = &metar.wx_string {
        let wx_upper = wx.to_uppercase();
        if wx_upper.contains("TS") || wx_upper.contains("FC") || wx_upper.contains("SQ") {
            return WeatherKeyword::Storm;
        }
        if wx_upper.contains("SN")
            || wx_upper.contains("PL")
            || wx_upper.contains("SG")
            || wx_upper.contains("IC")
            || wx_upper.contains("UP")
            || wx_upper.contains("FZ")
        {
            return WeatherKeyword::Snow;
        }
        if wx_upper.contains("RA") || wx_upper.contains("DZ") {
            return WeatherKeyword::Rain;
        }
        if wx_upper.contains("FG")
            || wx_upper.contains("BR")
            || wx_upper.contains("HZ")
            || wx_upper.contains("FU")
            || wx_upper.contains("DU")
        {
            return WeatherKeyword::Fog;
        }
    }

    // 2. Fallback to broad visibility/flight categories if provided
    if let Some(cat) = &metar.flight_category {
        if cat == "LIFR" || cat == "IFR" {
            // Very low ceiling / visibility = Fog or Dense clouds
            return WeatherKeyword::Fog;
        } else if cat == "MVFR" {
            // Marginal VFR = Cloud layer
            return WeatherKeyword::Cloudy;
        }
    }

    // 3. Wind Checks
    let wind = metar.wind_speed_kt.unwrap_or(0.0);
    let gust = metar.wind_gust_kt.unwrap_or(0.0);
    if wind >= 15.0 || gust >= 20.0 || gust > wind + 10.0 {
        return WeatherKeyword::Gusty;
    }

    // 4. Last fallback: Sky Cover
    if let Some(sky) = &metar.sky_cover {
        if sky == "OVC" || sky == "BKN" || sky == "VV" {
            return WeatherKeyword::Cloudy;
        }
    }

    // 5. Calm check (only if sky is clear)
    if wind <= 3.0 && gust == 0.0 {
        return WeatherKeyword::Calm;
    }

    WeatherKeyword::Clear
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_weather_keyword() {
        // Rain
        let rain = MetarRecord {
            wx_string: Some("-RA BR".into()),
            sky_cover: Some("OVC".into()),
            flight_category: Some("MVFR".into()),
            wind_speed_kt: None,
            wind_gust_kt: None,
        };
        assert_eq!(determine_weather_keyword(&rain), WeatherKeyword::Rain);

        // Storm
        let storm = MetarRecord {
            wx_string: Some("+TSRA".into()),
            sky_cover: Some("BKN".into()),
            flight_category: Some("IFR".into()),
            wind_speed_kt: Some(25.0),
            wind_gust_kt: Some(40.0),
        };
        assert_eq!(determine_weather_keyword(&storm), WeatherKeyword::Storm);

        // Snow (with freezing)
        let snow = MetarRecord {
            wx_string: Some("FZRA".into()),
            sky_cover: Some("OVC".into()),
            flight_category: Some("LIFR".into()),
            wind_speed_kt: None,
            wind_gust_kt: None,
        };
        assert_eq!(determine_weather_keyword(&snow), WeatherKeyword::Snow);

        // Fog
        let fog = MetarRecord {
            wx_string: Some("FG".into()),
            sky_cover: Some("VV".into()),
            flight_category: Some("LIFR".into()),
            wind_speed_kt: None,
            wind_gust_kt: None,
        };
        assert_eq!(determine_weather_keyword(&fog), WeatherKeyword::Fog);

        // Gusty (High wind, clear sky)
        let gusty = MetarRecord {
            wx_string: None,
            sky_cover: Some("CLR".into()),
            flight_category: Some("VFR".into()),
            wind_speed_kt: Some(18.0),
            wind_gust_kt: None,
        };
        assert_eq!(determine_weather_keyword(&gusty), WeatherKeyword::Gusty);

        // Calm (No wind, clear sky)
        let calm = MetarRecord {
            wx_string: None,
            sky_cover: Some("CLR".into()),
            flight_category: Some("VFR".into()),
            wind_speed_kt: Some(1.0),
            wind_gust_kt: None,
        };
        assert_eq!(determine_weather_keyword(&calm), WeatherKeyword::Calm);

        // Cloudy (No WX, but overcast)
        let cloudy = MetarRecord {
            wx_string: None,
            sky_cover: Some("OVC".into()),
            flight_category: Some("VFR".into()),
            wind_speed_kt: Some(10.0),
            wind_gust_kt: None,
        };
        assert_eq!(determine_weather_keyword(&cloudy), WeatherKeyword::Cloudy);

        // Clear
        let clear = MetarRecord {
            wx_string: None,
            sky_cover: Some("CLR".into()),
            flight_category: Some("VFR".into()),
            wind_speed_kt: Some(8.0),
            wind_gust_kt: None,
        };
        assert_eq!(determine_weather_keyword(&clear), WeatherKeyword::Clear);
    }
}
