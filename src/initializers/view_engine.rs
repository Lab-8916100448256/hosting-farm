use async_trait::async_trait;
use axum::{Extension, Router as AxumRouter};
use chrono::Local;
use fluent_templates::{ArcLoader, FluentLoader};
use loco_rs::{
    Error, Result,
    app::{AppContext, Initializer},
    controller::views::{ViewEngine, engines},
};
use std::collections::HashMap;
use tera::Value;
use tracing::info;

const I18N_DIR: &str = "assets/i18n";
const I18N_SHARED: &str = "assets/i18n/shared.ftl";
#[allow(clippy::module_name_repetitions)]
pub struct ViewEngineInitializer;

#[async_trait]
impl Initializer for ViewEngineInitializer {
    fn name(&self) -> String {
        "view-engine".to_string()
    }

    async fn after_routes(&self, router: AxumRouter, _ctx: &AppContext) -> Result<AxumRouter> {
        #[allow(unused_mut)]
        let mut tera_engine = engines::TeraView::build()?;

        // Register custom filters
        #[cfg(debug_assertions)]
        {
            let mut tera = tera_engine.tera.lock().expect("lock");

            // Register the slice filter
            tera.register_filter("slice", |value: &Value, args: &HashMap<String, Value>| {
                let start = args.get("start").and_then(|v| v.as_i64()).unwrap_or(0) as usize;

                let length = args.get("length").and_then(|v| v.as_i64()).unwrap_or(0) as usize;

                let s = value
                    .as_str()
                    .ok_or_else(|| tera::Error::msg("Value must be a string"))?;

                let end = (start + length).min(s.len());
                Ok(Value::String(s[start..end].to_string()))
            });

            // Register the date filter
            tera.register_filter(
                "date",
                |value: &Value, args: &HashMap<String, Value>| -> tera::Result<Value> {
                    let format = args
                        .get("format")
                        .and_then(|v| v.as_str())
                        .unwrap_or("%Y-%m-%d"); // Default to YYYY-MM-DD

                    // Try to convert the value to a chrono::DateTime<Utc> or chrono::DateTime<FixedOffset>
                    // Tera serializes chrono dates into a specific map structure or directly.
                    // We need to handle potential direct chrono::DateTime types if available,
                    // or serialized string representations.
                    let dt_result = match value {
                        Value::Object(map) => {
                            // Handle Tera's typical serialization of DateTime<Utc> or DateTime<FixedOffset>
                            if let (Some(Value::String(date_str)), Some(Value::String(fmt))) =
                                (map.get("date"), map.get("format"))
                            {
                                if fmt == "%Y-%m-%dT%H:%M:%S%.f%:z" {
                                    // DateTime<FixedOffset>
                                    chrono::DateTime::parse_from_rfc3339(date_str).map_err(|e| {
                                        tera::Error::msg(format!(
                                            "Failed to parse DateTime<FixedOffset> string: {}",
                                            e
                                        ))
                                    })
                                } else if fmt == "%Y-%m-%dT%H:%M:%S%.fZ" {
                                    // DateTime<Utc>
                                    chrono::DateTime::parse_from_rfc3339(date_str)
                                        .map_err(|e| {
                                            tera::Error::msg(format!(
                                                "Failed to parse DateTime<Utc> string: {}",
                                                e
                                            ))
                                        })
                                        .map(|dt| {
                                            dt.with_timezone(
                                                &chrono::FixedOffset::east_opt(0).unwrap(),
                                            )
                                        })
                                } else {
                                    Err(tera::Error::msg(format!(
                                        "Unsupported date format map: {}",
                                        fmt
                                    )))
                                }
                            } else {
                                Err(tera::Error::msg(
                                    "Value is object, but not a recognized date format map",
                                ))
                            }
                        }
                        Value::String(s) => {
                            // Attempt to parse common formats if it's just a string
                            chrono::DateTime::parse_from_rfc3339(s)
                                .or_else(|_| {
                                    chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f %z")
                                })
                                .or_else(|_| {
                                    chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                                })
                                .or_else(|_| {
                                    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d").map(
                                        |ndt| {
                                            ndt.and_local_timezone(chrono::Utc)
                                                .unwrap()
                                                .with_timezone(
                                                    &chrono::FixedOffset::east_opt(0).unwrap(),
                                                )
                                        },
                                    )
                                })
                                .map_err(|e| {
                                    tera::Error::msg(format!(
                                        "Failed to parse date string '{}': {}",
                                        s, e
                                    ))
                                })
                        }
                        _ => Err(tera::Error::msg(
                            "Date filter input must be a date object or a string",
                        )),
                    };

                    let dt = dt_result?;
                    Ok(Value::String(dt.format(format).to_string()))
                },
            );

            // Register the now() function
            tera.register_function("now", |_args: &HashMap<String, Value>| {
                let now = Local::now();
                Ok(Value::String(now.to_string()))
            });
        }

        #[cfg(not(debug_assertions))]
        {
            // Register the slice filter
            tera_engine.tera.register_filter(
                "slice",
                |value: &Value, args: &HashMap<String, Value>| {
                    let start = args.get("start").and_then(|v| v.as_i64()).unwrap_or(0) as usize;

                    let length = args.get("length").and_then(|v| v.as_i64()).unwrap_or(0) as usize;

                    let s = value
                        .as_str()
                        .ok_or_else(|| tera::Error::msg("Value must be a string"))?;

                    let end = (start + length).min(s.len());
                    Ok(Value::String(s[start..end].to_string()))
                },
            );

            // Register the date filter
            tera_engine.tera.register_filter(
                "date",
                |value: &Value, args: &HashMap<String, Value>| -> tera::Result<Value> {
                    let format = args
                        .get("format")
                        .and_then(|v| v.as_str())
                        .unwrap_or("%Y-%m-%d"); // Default to YYYY-MM-DD

                    // Try to convert the value to a chrono::DateTime<Utc> or chrono::DateTime<FixedOffset>
                    // Tera serializes chrono dates into a specific map structure or directly.
                    // We need to handle potential direct chrono::DateTime types if available,
                    // or serialized string representations.
                    let dt_result = match value {
                        Value::Object(map) => {
                            // Handle Tera's typical serialization of DateTime<Utc> or DateTime<FixedOffset>
                            if let (Some(Value::String(date_str)), Some(Value::String(fmt))) =
                                (map.get("date"), map.get("format"))
                            {
                                if fmt == "%Y-%m-%dT%H:%M:%S%.f%:z" {
                                    // DateTime<FixedOffset>
                                    chrono::DateTime::parse_from_rfc3339(date_str).map_err(|e| {
                                        tera::Error::msg(format!(
                                            "Failed to parse DateTime<FixedOffset> string: {}",
                                            e
                                        ))
                                    })
                                } else if fmt == "%Y-%m-%dT%H:%M:%S%.fZ" {
                                    // DateTime<Utc>
                                    chrono::DateTime::parse_from_rfc3339(date_str)
                                        .map_err(|e| {
                                            tera::Error::msg(format!(
                                                "Failed to parse DateTime<Utc> string: {}",
                                                e
                                            ))
                                        })
                                        .map(|dt| {
                                            dt.with_timezone(
                                                &chrono::FixedOffset::east_opt(0).unwrap(),
                                            )
                                        })
                                } else {
                                    Err(tera::Error::msg(format!(
                                        "Unsupported date format map: {}",
                                        fmt
                                    )))
                                }
                            } else {
                                Err(tera::Error::msg(
                                    "Value is object, but not a recognized date format map",
                                ))
                            }
                        }
                        Value::String(s) => {
                            // Attempt to parse common formats if it's just a string
                            chrono::DateTime::parse_from_rfc3339(s)
                                .or_else(|_| {
                                    chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f %z")
                                })
                                .or_else(|_| {
                                    chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                                })
                                .or_else(|_| {
                                    chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d").map(
                                        |ndt| {
                                            ndt.and_local_timezone(chrono::Utc)
                                                .unwrap()
                                                .with_timezone(
                                                    &chrono::FixedOffset::east_opt(0).unwrap(),
                                                )
                                        },
                                    )
                                })
                                .map_err(|e| {
                                    tera::Error::msg(format!(
                                        "Failed to parse date string '{}': {}",
                                        s, e
                                    ))
                                })
                        }
                        _ => Err(tera::Error::msg(
                            "Date filter input must be a date object or a string",
                        )),
                    };

                    let dt = dt_result?;
                    Ok(Value::String(dt.format(format).to_string()))
                },
            );

            // Register the now() function
            tera_engine
                .tera
                .register_function("now", |_args: &HashMap<String, Value>| {
                    let now = Local::now();
                    Ok(Value::String(now.to_string()))
                });
        }

        if std::path::Path::new(I18N_DIR).exists() {
            let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
                .shared_resources(Some(&[I18N_SHARED.into()]))
                .customize(|bundle| bundle.set_use_isolating(false))
                .build()
                .map_err(|e| Error::string(&e.to_string()))?;
            #[cfg(debug_assertions)]
            tera_engine
                .tera
                .lock()
                .expect("lock")
                .register_function("t", FluentLoader::new(arc));

            #[cfg(not(debug_assertions))]
            tera_engine
                .tera
                .register_function("t", FluentLoader::new(arc));
            info!("locales loaded");
        }

        Ok(router.layer(Extension(ViewEngine::from(tera_engine))))
    }
}
