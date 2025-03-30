use loco_rs::{
    app::AppContext,
    controller::format,
    Result,
};
use axum::response::Response;
use tera::{Context, Value};
use chrono::Local;

// Path to view templates
const VIEWS_DIR: &str = "assets/views";

// Create Tera instance (lazy_static would be better in a real app)
fn create_tera() -> Result<tera::Tera> {
    let glob_pattern = format!("{}/**/*.html", VIEWS_DIR);
    let mut tera = match tera::Tera::new(&glob_pattern) {
        Ok(t) => t,
        Err(e) => return Err(loco_rs::Error::string(&format!("Failed to initialize Tera: {}", e))),
    };

    // Register the now() function
    tera.register_function("now", |_args: &std::collections::HashMap<String, Value>| {
        let now = Local::now();
        Ok(Value::String(now.to_string()))
    });

    // Register the date filter
    tera.register_filter("date", |value: &Value, args: &std::collections::HashMap<String, Value>| {
        let format = args.get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("%Y");
        
        let date = value.as_str()
            .ok_or_else(|| tera::Error::msg("Value must be a string"))?;
            
        let datetime = chrono::DateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S%.f %z")
            .map_err(|e| tera::Error::msg(format!("Failed to parse date: {}", e)))?;
            
        Ok(Value::String(datetime.format(format).to_string()))
    });

    // Register the slice filter
    tera.register_filter("slice", |value: &Value, args: &std::collections::HashMap<String, Value>| {
        let start = args.get("start")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;
            
        let length = args.get("length")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as usize;
            
        let s = value.as_str()
            .ok_or_else(|| tera::Error::msg("Value must be a string"))?;
            
        let end = (start + length).min(s.len());
        Ok(Value::String(s[start..end].to_string()))
    });

    Ok(tera)
}

/// Renders a template with the given context
pub fn render_template(_ctx: &AppContext, template_name: &str, context: Context) -> Result<Response> {
    // Initialize Tera
    let tera = create_tera()?;
    
    // Render the template
    let rendered = tera.render(template_name, &context)
        .map_err(|e| loco_rs::Error::string(&format!("Failed to render template: {:?}", e)))?;
    
    format::html(&rendered)
}