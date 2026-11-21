//! User settings: validated by the schema in medore-core, saved as JSON
//! atomically (only values that differ from the defaults are written).

use parking_lot::RwLock;
use serde_json::{Map, Value};

pub use crate::core::settings_schema::{coerce, defaults};

pub struct Settings {
    values: RwLock<Map<String, Value>>,
}

impl Settings {
    pub fn load() -> Settings {
        let mut values = defaults().clone();
        let path = crate::paths::data_file("settings.json");
        if let Ok(text) = std::fs::read_to_string(&path) {
            match serde_json::from_str::<Map<String, Value>>(&text) {
                Ok(saved) => {
                    for (k, v) in saved {
                        if let Ok(v) = coerce(&k, v) {
                            values.insert(k, v);
                        }
                    }
                }
                Err(e) => {
                    log::warn!("settings.json unreadable ({e}); keeping a copy and using defaults");
                    let _ = std::fs::copy(&path, path.with_extension("broken.json"));
                }
            }
        }
        Settings { values: RwLock::new(values) }
    }

    pub fn get(&self, key: &str) -> Value {
        self.values.read().get(key).cloned().unwrap_or(Value::Null)
    }
    pub fn str(&self, key: &str) -> String {
        self.get(key).as_str().unwrap_or("").to_string()
    }
    pub fn bool(&self, key: &str) -> bool {
        self.get(key).as_bool().unwrap_or(false)
    }
    pub fn f64(&self, key: &str) -> f64 {
        self.get(key).as_f64().unwrap_or(0.0)
    }
    pub fn strings(&self, key: &str) -> Vec<String> {
        self.get(key).as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default()
    }
    pub fn all(&self) -> Map<String, Value> {
        self.values.read().clone()
    }

    pub fn set(&self, key: &str, value: Value) -> Result<Value, String> {
        let v = coerce(key, value)?;
        {
            let mut g = self.values.write();
            if g.get(key) == Some(&v) {
                return Ok(v);
            }
            g.insert(key.to_string(), v.clone());
        }
        self.save();
        Ok(v)
    }

    pub fn reset(&self, key: &str) -> Result<Value, String> {
        let d = defaults().get(key).cloned().ok_or_else(|| format!("unknown setting {key}"))?;
        self.set(key, d)
    }

    fn save(&self) {
        let snapshot = self.values.read().clone();
        // only store what differs from the defaults
        let diff: Map<String, Value> = snapshot.into_iter().filter(|(k, v)| defaults().get(k) != Some(v)).collect();
        let text = serde_json::to_vec_pretty(&diff).unwrap_or_default();
        if let Err(e) = crate::paths::write_atomic(&crate::paths::data_file("settings.json"), &text) {
            log::error!("cannot save settings: {e}");
        }
    }
}

