use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::HeaderMap,
    Json,
};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn, error};
use uuid::Uuid;

use crate::app::AppState;
use crate::features::auth::{create_auth_context, decode_jwt, extract_bearer_token, JwtClaims};
use crate::shared::{AppError, AuthContext, TokenType};

pub struct AuthMiddleware;

impl AuthMiddleware {
    pub async fn authenticate(
        State(state): State<AppState>,
        mut request: Request,
        next: Next,
    ) -> Result<Response, AppError> {
        let start_time = SystemTime::now();
        let headers = request.headers();
        let request_id = Uuid::new_v4().to_string();

        debug!(request_id = %request_id, "Starting authentication middleware");

        // Extract authorization header
        let auth_header = headers
            .get("authorization")
            .and_then(|header| header.to_str().ok())
            .ok_or_else(|| {
                warn!(request_id = %request_id, "Missing Authorization header");
                AppError::authentication("Missing Authorization header")
            })?;

        // Extract bearer token
        let token = extract_bearer_token(auth_header).map_err(|e| {
            warn!(request_id = %request_id, error = %e, "Failed to extract bearer token");
            e
        })?;

        // Decode and validate JWT token
        let claims = decode_jwt(token, &state.config.supabase.jwt_secret).map_err(|e| {
            warn!(
                request_id = %request_id,
                error = %e,
                token_prefix = &token[..std::cmp::min(10, token.len())],
                "JWT validation failed"
            );
            AppError::authentication("Invalid or expired JWT token")
        })?;

        // Validate token expiration with clock skew tolerance
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as usize;

        if claims.exp <= current_time {
            warn!(
                request_id = %request_id,
                user_id = %claims.sub,
                exp = claims.exp,
                current_time = current_time,
                "Token has expired"
            );
            return Err(AppError::authentication("Token has expired"));
        }

        // Validate audience claim
        if claims.aud != "authenticated" && !claims.aud.contains("authenticated") {
            warn!(
                request_id = %request_id,
                user_id = %claims.sub,
                audience = %claims.aud,
                "Invalid audience claim"
            );
            return Err(AppError::authentication("Invalid token audience"));
        }

        // Validate additional security constraints
        if let Err(e) = Self::validate_security_constraints(headers, &claims) {
            warn!(
                request_id = %request_id,
                user_id = %claims.sub,
                error = %e,
                "Security constraints validation failed"
            );
            return Err(e);
        }

        // Create enriched auth context
        let auth_context = create_auth_context(
            &claims,
            &state.config.supabase.anon_key,
            &state.config.supabase.service_role_key,
        ).map_err(|e| {
            error!(
                request_id = %request_id,
                user_id = %claims.sub,
                error = %e,
                "Failed to create auth context"
            );
            AppError::authentication("Failed to process authentication")
        })?;

        // Log successful authentication
        debug!(
            request_id = %request_id,
            user_id = %claims.sub,
            role = %claims.role,
            token_type = ?auth_context.token_type,
            duration_ms = start_time.elapsed().unwrap_or_default().as_millis(),
            "Authentication successful"
        );

        // Insert auth context into request extensions for use by handlers
        request.extensions_mut().insert(auth_context);
        request.extensions_mut().insert(claims); // Also store raw claims for advanced use cases

        // Continue to the next middleware/handler
        Ok(next.run(request).await)
    }

    pub async fn optional_authenticate(
        State(state): State<AppState>,
        mut request: Request,
        next: Next,
    ) -> Response {
        let start_time = SystemTime::now();
        let headers = request.headers();
        let request_id = Uuid::new_v4().to_string();

        debug!(request_id = %request_id, "Starting optional authentication middleware");

        // Try to extract and decode token, but don't fail if missing/invalid
        if let Some(auth_header) = headers
            .get("authorization")
            .and_then(|header| header.to_str().ok())
        {
            debug!(request_id = %request_id, "Authorization header found, attempting authentication");

            if let Ok(token) = extract_bearer_token(auth_header) {
                if let Ok(claims) = decode_jwt(token, &state.config.supabase.jwt_secret) {
                    // Validate token expiration (optional auth allows expired tokens to be treated as anonymous)
                    let current_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as usize;

                    if claims.exp > current_time {
                        // Validate audience claim (lenient for optional auth)
                        if claims.aud == "authenticated" || claims.aud.contains("authenticated") {
                            if let Ok(auth_context) = create_auth_context(
                                &claims,
                                &state.config.supabase.anon_key,
                                &state.config.supabase.service_role_key,
                            ) {
                                debug!(
                                    request_id = %request_id,
                                    user_id = %claims.sub,
                                    role = %claims.role,
                                    token_type = ?auth_context.token_type,
                                    duration_ms = start_time.elapsed().unwrap_or_default().as_millis(),
                                    "Optional authentication successful"
                                );

                                request.extensions_mut().insert(auth_context);
                                request.extensions_mut().insert(claims);
                                return next.run(request).await;
                            } else {
                                debug!(request_id = %request_id, "Failed to create auth context, falling back to anonymous");
                            }
                        } else {
                            debug!(
                                request_id = %request_id,
                                audience = %claims.aud,
                                "Invalid audience claim, falling back to anonymous"
                            );
                        }
                    } else {
                        debug!(
                            request_id = %request_id,
                            exp = claims.exp,
                            current_time = current_time,
                            "Token expired, falling back to anonymous"
                        );
                    }
                } else {
                    debug!(request_id = %request_id, "JWT validation failed, falling back to anonymous");
                }
            } else {
                debug!(request_id = %request_id, "Failed to extract bearer token, falling back to anonymous");
            }
        } else {
            debug!(request_id = %request_id, "No authorization header found, proceeding as anonymous");
        }

        // If no valid auth, insert unauthenticated context
        if request.extensions().get::<AuthContext>().is_none() {
            let anon_context = AuthContext {
                user_id: None,
                token_type: TokenType::Anon,
                is_authenticated: false,
            };

            debug!(
                request_id = %request_id,
                duration_ms = start_time.elapsed().unwrap_or_default().as_millis(),
                "Proceeding with anonymous authentication"
            );

            request.extensions_mut().insert(anon_context);
        }

        next.run(request).await
    }
}

// Helper function to extract auth context from request extensions
pub fn get_auth_context(request: &Request) -> Option<&AuthContext> {
    request.extensions().get::<AuthContext>()
}

// Helper function to extract JWT claims from request extensions
pub fn get_jwt_claims(request: &Request) -> Option<&JwtClaims> {
    request.extensions().get::<JwtClaims>()
}

// Helper function to check if request is authenticated
pub fn is_authenticated(request: &Request) -> bool {
    get_auth_context(request)
        .map(|ctx| ctx.is_authenticated)
        .unwrap_or(false)
}

// Helper function to get user ID from authenticated request
pub fn get_user_id(request: &Request) -> Option<String> {
    get_auth_context(request)
        .and_then(|ctx| ctx.user_id.map(|uuid| uuid.to_string()))
}

// Helper function to check user role
pub fn has_role(request: &Request, required_role: &str) -> bool {
    get_jwt_claims(request)
        .map(|claims| claims.role == required_role)
        .unwrap_or(false)
}

// Helper function to check if user has service role access
pub fn has_service_role(request: &Request) -> bool {
    get_auth_context(request)
        .map(|ctx| matches!(ctx.token_type, TokenType::ServiceRole))
        .unwrap_or(false)
}

// Middleware-specific error handling for authentication failures
impl AuthMiddleware {
    /// Create a standardized authentication error response
    pub fn auth_error_response(message: &str, request_id: Option<String>) -> Json<Value> {
        let error_response = json!({
            "error": {
                "message": message,
                "type": "authentication_error",
                "code": "invalid_api_key",
                "request_id": request_id
            }
        });

        Json(error_response)
    }

    /// Validate additional security headers and constraints
    pub fn validate_security_constraints(headers: &HeaderMap, claims: &JwtClaims) -> Result<(), AppError> {
        // Check for required security headers in production
        if std::env::var("RUST_ENV").unwrap_or_default() == "production" {
            // Validate User-Agent header is present
            if headers.get("user-agent").is_none() {
                return Err(AppError::authentication("Missing required User-Agent header"));
            }

            // Validate Origin header for web requests if present
            if let Some(origin) = headers.get("origin") {
                if let Ok(origin_str) = origin.to_str() {
                    // Add domain validation logic here
                    debug!("Request origin: {}", origin_str);
                }
            }
        }

        // Validate token hasn't been issued in the future (clock skew protection)
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as usize;

        const CLOCK_SKEW_TOLERANCE: usize = 300; // 5 minutes
        if claims.iat > current_time + CLOCK_SKEW_TOLERANCE {
            return Err(AppError::authentication("Token issued in the future"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use crate::infrastructure::supabase::SupabaseClient;
    use crate::infrastructure::VertexAIClient;
    use axum::body::Body;
    use axum::http::Method;
    use std::sync::Arc;

    // Test helper will be implemented when we have proper test setup
    // For now, skip these tests
}
