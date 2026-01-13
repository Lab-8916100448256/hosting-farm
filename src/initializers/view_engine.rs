use async_trait::async_trait;
use axum::{Extension, Router as AxumRouter};
use chrono::Local;
use fluent_templates::{ArcLoader, FluentLoader};
use loco_rs::{
    Result,
    app::{AppContext, Initializer},
    prelude::*,
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
        let mut tera_engine = loco_rs::controller::views::engines::TeraView::build()?;

        tera_engine = tera_engine.post_process(move |tera| {
            // Register custom filters
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

                    let dt_result = match value {
                        Value::Object(map) => {
                            if let (Some(Value::String(date_str)), Some(Value::String(fmt))) =
                                (map.get("date"), map.get("format"))
                            {
                                if fmt == "%Y-%m-%dT%H:%M:%S%.f%:z" {
                                    chrono::DateTime::parse_from_rfc3339(date_str).map_err(|e| {
                                        tera::Error::msg(format!(
                                            "Failed to parse DateTime<FixedOffset> string: {}",
                                            e
                                        ))
                                    })
                                } else if fmt == "%Y-%m-%dT%H:%M:%S%.fZ" {
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
                        Value::String(s) => chrono::DateTime::parse_from_rfc3339(s)
                            .or_else(|_| {
                                chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f %z")
                            })
                            .or_else(|_| chrono::DateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
                            .or_else(|_| {
                                chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d").map(|ndt| {
                                    ndt.and_local_timezone(chrono::Utc)
                                        .unwrap()
                                        .with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
                                })
                            })
                            .map_err(|e| {
                                tera::Error::msg(format!(
                                    "Failed to parse date string '{}': {}",
                                    s, e
                                ))
                            }),
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

            if std::path::Path::new(I18N_DIR).exists() {
                let arc = ArcLoader::builder(&I18N_DIR, unic_langid::langid!("en-US"))
                    .shared_resources(Some(&[I18N_SHARED.into()]))
                    .customize(|bundle| bundle.set_use_isolating(false))
                    .build()
                    .map_err(|e| tera::Error::msg(e.to_string()))?;

                tera.register_function("t", FluentLoader::new(arc));
                info!("locales loaded");
            }

            Ok(())
        })?;

        Ok(router.layer(Extension(ViewEngine::new(tera_engine))))
    }
}
