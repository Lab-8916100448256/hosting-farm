use loco_rs::{
    app::AppContext,
    controller::{format, views::ViewEngine},
    Result,
};
use axum::response::Response;
use tera::Context;

/// Renders a template with the given context
pub fn render_template(ctx: &AppContext, template_name: &str, context: Context) -> Result<Response> {
    let view_engine = ctx.get_extension::<ViewEngine>()
        .ok_or_else(|| loco_rs::Error::string("ViewEngine extension not found"))?;
    
    let rendered = view_engine.render(template_name, &context)
        .map_err(|e| loco_rs::Error::string(&format!("Failed to render template: {}", e)))?;
    
    format::html(&rendered)
} 