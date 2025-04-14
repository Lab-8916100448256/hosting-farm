use axum::response::Response;
use loco_rs::prelude::*;
use loco_rs::{controller::format, Result};
use serde::Serialize;

// Path to view templates
// const VIEWS_DIR: &str = "assets/views";

/// Renders a template with the given context
pub fn render_template<V, S>(v: &V, template_name: &str, context: S) -> Result<Response>
where
    V: ViewRenderer,
    S: Serialize,
{
    format::render().view(v, template_name, context)
}
