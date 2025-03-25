use loco_rs::{
    app::AppContext,
    controller::format,
    Result,
};
use axum::response::Response;
use tera::Context;
use std::sync::Arc;

// Path to view templates
const VIEWS_DIR: &str = "assets/views";

// Create Tera instance (lazy_static would be better in a real app)
fn create_tera() -> Result<tera::Tera> {
    let glob_pattern = format!("{}/**/*.html.tera", VIEWS_DIR);
    match tera::Tera::new(&glob_pattern) {
        Ok(t) => Ok(t),
        Err(e) => Err(loco_rs::Error::string(&format!("Failed to initialize Tera: {}", e))),
    }
}

/// Renders a template with the given context
pub fn render_template(_ctx: &AppContext, template_name: &str, context: Context) -> Result<Response> {
    // Initialize Tera
    let mut tera = create_tera()?;
    
    // Render the template
    let rendered = tera.render(template_name, &context)
        .map_err(|e| loco_rs::Error::string(&format!("Failed to render template: {}", e)))?;
    
    format::html(&rendered)
}