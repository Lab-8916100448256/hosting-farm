//! Axum middleware for validating token header

use std::collections::HashMap;

use axum::{
    extract::{FromRef, FromRequestParts, Query},
    http::{request::Parts, HeaderMap},
};
use axum_extra::extract::cookie;
use serde::{Deserialize, Serialize};

use loco_rs::{
    app::AppContext, auth, config::JWT as JWTConfig, errors::Error, model::Authenticable,
    Result as LocoResult,
};

// ---------------------------------------
//
// JWT Auth extractor
//
// ---------------------------------------

// Define constants for token prefix and authorization header
const TOKEN_PREFIX: &str = "Bearer ";
const AUTH_HEADER: &str = "authorization";

// Define a struct to represent user authentication information serialized
// to/from JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct JWTWithUserOpt<T: Authenticable> {
    pub claims: Option<auth::jwt::UserClaims>,
    pub user: Option<T>,
}

// Implement the FromRequestParts trait for the Auth struct
impl<S, T> FromRequestParts<S> for JWTWithUserOpt<T>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
    T: Authenticable,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        let ctx: AppContext = AppContext::from_ref(state);

        match get_jwt_from_config(&ctx) {
            Ok(jwt_config) => {
                match extract_token(jwt_config, parts) {
                    Ok(token) => {
                        let jwt_secret = ctx.config.get_jwt_config()?;
                        match auth::jwt::JWT::new(&jwt_secret.secret).validate(&token) {
                            Ok(claims) => {
                                let user = T::find_by_claims_key(&ctx.db, &claims.claims.pid)
                                    .await;
                                match user {
                                    Ok(user) => {
                                        Ok(Self {
                                            claims: Some(claims.claims),
                                            user: Some(user),
                                        })        
                                    }
                                    Err(_) => {
                                        // ToDo: log error ?
                                        Ok(Self {
                                            claims: Some(claims.claims),
                                            user: None,
                                        }) 
                                    }
                                }
                            }
                            Err(_err) => {
                                // ToDo: log error ?
                                Ok(Self {
                                    claims: None,
                                    user: None,
                                }) 
                            }
                        }        
        
                    }
                    Err(_) => {
                        // ToDo: log error ?
                        Ok(Self {
                            claims: None,
                            user: None,
                        }) 
                    }
                }
            }
            Err(_) => {
                // ToDo: log error ?
                Ok(Self {
                    claims: None,
                    user: None,
                }) 
            }
        }
    }
}

// Define a struct to represent user authentication information serialized
// to/from JSON
#[derive(Debug, Deserialize, Serialize)]
pub struct JWTOpt {
    pub claims: Option<auth::jwt::UserClaims>,
}

// Implement the FromRequestParts trait for the Auth struct
impl<S> FromRequestParts<S> for JWTOpt
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let ctx: AppContext = AppContext::from_ref(state); // change to ctx

        match get_jwt_from_config(&ctx) {
            Ok(jwt_config) => {
                match extract_token(jwt_config, parts) {
                    Ok(token) => {
                        let jwt_secret = ctx.config.get_jwt_config()?;
                        match auth::jwt::JWT::new(&jwt_secret.secret).validate(&token) {
                            Ok(claims) => {
                                Ok(Self {
                                    claims: Some(claims.claims),
                                })                           }
                            Err(_err) => {
                                // ToDo: log error ?
                                Ok(Self {
                                    claims: None,
                                })   
                            }
                        }        
        
                    }
                    Err(_) => {
                        // ToDo: log error ?
                        Ok(Self {
                            claims: None,
                        })   
                    }
                }
            }
            Err(_) => {
                // ToDo: log error ?
                Ok(Self {
                    claims: None,
                })   
            }
        }

    }
}

/// extract JWT token from context configuration
///
/// # Errors
/// Return an error when JWT token not configured
fn get_jwt_from_config(ctx: &AppContext) -> LocoResult<&JWTConfig> {
    ctx.config
        .auth
        .as_ref()
        .ok_or_else(|| Error::string("auth not configured"))?
        .jwt
        .as_ref()
        .ok_or_else(|| Error::string("JWT token not configured"))
}
/// extract token from the configured jwt location settings
fn extract_token(jwt_config: &JWTConfig, parts: &Parts) -> LocoResult<String> {
    #[allow(clippy::match_wildcard_for_single_variants)]
    match jwt_config
        .location
        .as_ref()
        .unwrap_or(&loco_rs::config::JWTLocation::Bearer)
    {
        loco_rs::config::JWTLocation::Query { name } => extract_token_from_query(name, parts),
        loco_rs::config::JWTLocation::Cookie { name } => extract_token_from_cookie(name, parts),
        loco_rs::config::JWTLocation::Bearer => extract_token_from_header(&parts.headers)
            .map_err(|e| Error::Unauthorized(e.to_string())),
    }
}
/// Function to extract a token from the authorization header
///
/// # Errors
///
/// When token is not valid or out found
pub fn extract_token_from_header(headers: &HeaderMap) -> LocoResult<String> {
    Ok(headers
        .get(AUTH_HEADER)
        .ok_or_else(|| Error::Unauthorized(format!("header {AUTH_HEADER} token not found")))?
        .to_str()
        .map_err(|err| Error::Unauthorized(err.to_string()))?
        .strip_prefix(TOKEN_PREFIX)
        .ok_or_else(|| Error::Unauthorized(format!("error strip {AUTH_HEADER} value")))?
        .to_string())
}

/// Extract a token value from cookie
///
/// # Errors
/// when token value from cookie is not found
pub fn extract_token_from_cookie(name: &str, parts: &Parts) -> LocoResult<String> {
    // LogoResult
    let jar: cookie::CookieJar = cookie::CookieJar::from_headers(&parts.headers);
    Ok(jar
        .get(name)
        .ok_or(Error::Unauthorized("token is not found".to_string()))?
        .to_string()
        .strip_prefix(&format!("{name}="))
        .ok_or_else(|| Error::Unauthorized("error strip value".to_string()))?
        .to_string())
}
/// Extract a token value from query
///
/// # Errors
/// when token value from cookie is not found
pub fn extract_token_from_query(name: &str, parts: &Parts) -> LocoResult<String> {
    // LogoResult
    let parameters: Query<HashMap<String, String>> =
        Query::try_from_uri(&parts.uri).map_err(|err| Error::Unauthorized(err.to_string()))?;
    parameters
        .get(name)
        .cloned()
        .ok_or_else(|| Error::Unauthorized(format!("`{name}` query parameter not found")))
}

// ---------------------------------------
//
// API Token Auth / Extractor
//
// ---------------------------------------
#[derive(Debug, Deserialize, Serialize)]
// Represents the data structure for the API token.
pub struct ApiTokenOpt<T: Authenticable> {
    pub user: Option<T>,
}

// Implementing the `FromRequestParts` trait for `ApiToken` to enable extracting
// it from the request.
impl<S, T> FromRequestParts<S> for ApiTokenOpt<T>
where
    AppContext: FromRef<S>,
    S: Send + Sync,
    T: Authenticable,
{
    type Rejection = Error;

    // Extracts `ApiToken` from the request parts.
    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        // Extract API key from the request header.
        match extract_token_from_header(&parts.headers) {
            Ok(api_key) => {
                // Convert the state reference to the application context.
                let state: AppContext = AppContext::from_ref(state);

                // Retrieve user information based on the API key from the database.
                match T::find_by_api_key(&state.db, &api_key).await {
                    Ok(user) => {
                        Ok(Self { user: Some(user) })
                    }
                    Err(_) => {
                        //ToDo: Log error?
                        Ok(Self { user: None })
                    }
                }
            }
            Err(_) => {
                //ToDo: Log error?
                Ok(Self { user: None })
            }
        }
    }
}

#[cfg(test)]
mod tests {
  //ToDo: Tests
}
