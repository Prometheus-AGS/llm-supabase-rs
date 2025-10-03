use crate::shared::{AppError, AppResult, AuthContext, TokenType};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub aud: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_metadata: Option<serde_json::Value>,
}

pub fn decode_jwt(token: &str, secret: &str) -> AppResult<JwtClaims> {
    let key = DecodingKey::from_secret(secret.as_ref());

    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    validation.validate_aud = false; // Supabase has specific audience handling

    let token_data = decode::<JwtClaims>(token, &key, &validation)
        .map_err(|e| AppError::authentication(format!("Invalid JWT token: {}", e)))?;

    Ok(token_data.claims)
}

pub fn create_auth_context(
    claims: &JwtClaims,
    _anon_key: &str,
    _service_role_key: &str,
) -> AppResult<AuthContext> {
    // Check if this is an anon token by comparing the role
    if claims.role == "anon" {
        return Ok(AuthContext {
            user_id: None,
            token_type: TokenType::Anon,
            is_authenticated: true,
        });
    }

    // Check if this is a service role token
    if claims.role == "service_role" {
        return Ok(AuthContext {
            user_id: None,
            token_type: TokenType::ServiceRole,
            is_authenticated: true,
        });
    }

    // This should be a user token
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|e| AppError::authentication(format!("Invalid user ID in token: {}", e)))?;

    Ok(AuthContext {
        user_id: Some(user_id),
        token_type: TokenType::User(user_id),
        is_authenticated: true,
    })
}

pub fn extract_bearer_token(auth_header: &str) -> AppResult<&str> {
    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::authentication(
            "Authorization header must start with 'Bearer '",
        ));
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix
    if token.is_empty() {
        return Err(AppError::authentication("Empty bearer token"));
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header};

    fn create_test_claims() -> JwtClaims {
        let now = Utc::now();
        JwtClaims {
            sub: Uuid::new_v4().to_string(),
            aud: "authenticated".to_string(),
            exp: (now + Duration::hours(1)).timestamp() as usize,
            iat: now.timestamp() as usize,
            iss: "supabase".to_string(),
            role: "authenticated".to_string(),
            email: Some("test@example.com".to_string()),
            app_metadata: None,
            user_metadata: None,
        }
    }

    #[test]
    fn test_decode_valid_jwt() {
        let secret = "test-secret";
        let claims = create_test_claims();

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_ref()),
        )
        .unwrap();

        let decoded = decode_jwt(&token, secret).unwrap();
        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.role, claims.role);
    }

    #[test]
    fn test_extract_bearer_token() {
        assert_eq!(extract_bearer_token("Bearer abc123").unwrap(), "abc123");
        assert!(extract_bearer_token("abc123").is_err());
        assert!(extract_bearer_token("Bearer ").is_err());
    }
}
