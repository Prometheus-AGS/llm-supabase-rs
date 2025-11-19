//! AWS Bedrock authentication implementation
//!
//! This module handles AWS SigV4 authentication for Bedrock API requests.
//! It supports multiple authentication methods including environment variables,
//! credentials files, IAM roles, and explicit credentials.

use anyhow::{anyhow, Result};
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{debug, trace, warn};

use super::types::{BedrockAuthMethod, BedrockConfig, BedrockError};

/// AWS credentials structure
#[derive(Debug, Clone)]
pub struct AwsCredentials {
    /// AWS access key ID
    pub access_key: String,
    
    /// AWS secret access key
    pub secret_key: String,
    
    /// Optional session token for temporary credentials
    pub session_token: Option<String>,
    
    /// Expiration time for temporary credentials
    pub expiration: Option<DateTime<Utc>>,
    
    /// Region for the credentials
    pub region: String,
}

/// AWS SigV4 signer for Bedrock requests
#[derive(Debug, Clone)]
pub struct BedrockAuth {
    /// AWS credentials
    credentials: AwsCredentials,
    
    /// Authentication method used
    auth_method: BedrockAuthMethod,
    
    /// Service name (always "bedrock" for Bedrock API)
    service: String,
}

/// AWS credentials file structure
#[derive(Debug, Deserialize)]
struct AwsCredentialsFile {
    #[serde(flatten)]
    profiles: HashMap<String, AwsProfile>,
}

/// AWS profile structure
#[derive(Debug, Deserialize)]
struct AwsProfile {
    aws_access_key_id: Option<String>,
    aws_secret_access_key: Option<String>,
    aws_session_token: Option<String>,
    region: Option<String>,
}

/// EC2 instance metadata credentials
#[derive(Debug, Deserialize)]
struct Ec2Credentials {
    #[serde(rename = "AccessKeyId")]
    access_key_id: String,
    
    #[serde(rename = "SecretAccessKey")]
    secret_access_key: String,
    
    #[serde(rename = "Token")]
    token: Option<String>,
    
    #[serde(rename = "Expiration")]
    expiration: String,
}

impl BedrockAuth {
    /// Create new Bedrock authentication from configuration
    pub async fn new(config: &BedrockConfig) -> Result<Self> {
        debug!("Initializing AWS Bedrock authentication");
        
        let credentials = Self::load_credentials(config).await?;
        
        Ok(Self {
            credentials,
            auth_method: config.auth_method.clone(),
            service: "bedrock".to_string(),
        })
    }
    
    /// Load AWS credentials based on configuration
    async fn load_credentials(config: &BedrockConfig) -> Result<AwsCredentials> {
        match config.auth_method {
            BedrockAuthMethod::Explicit => {
                Self::load_explicit_credentials(config)
            }
            BedrockAuthMethod::EnvironmentVariables => {
                Self::load_env_credentials(config)
            }
            BedrockAuthMethod::CredentialsFile => {
                Self::load_file_credentials(config).await
            }
            BedrockAuthMethod::InstanceProfile => {
                Self::load_instance_profile_credentials(config).await
            }
            BedrockAuthMethod::EcsTaskRole => {
                Self::load_ecs_task_credentials(config).await
            }
            BedrockAuthMethod::LambdaRole => {
                Self::load_lambda_credentials(config).await
            }
            BedrockAuthMethod::Default => {
                Self::load_default_credentials(config).await
            }
        }
    }
    
    /// Load explicit credentials from configuration
    fn load_explicit_credentials(config: &BedrockConfig) -> Result<AwsCredentials> {
        let access_key = config.access_key.as_ref()
            .ok_or_else(|| anyhow!("Access key not provided for explicit authentication"))?;
            
        let secret_key = config.secret_key.as_ref()
            .ok_or_else(|| anyhow!("Secret key not provided for explicit authentication"))?;
            
        Ok(AwsCredentials {
            access_key: access_key.clone(),
            secret_key: secret_key.clone(),
            session_token: config.session_token.clone(),
            expiration: None,
            region: config.region.clone(),
        })
    }
    
    /// Load credentials from environment variables
    fn load_env_credentials(config: &BedrockConfig) -> Result<AwsCredentials> {
        let access_key = env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| anyhow!("AWS_ACCESS_KEY_ID environment variable not found"))?;
            
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| anyhow!("AWS_SECRET_ACCESS_KEY environment variable not found"))?;
            
        let session_token = env::var("AWS_SESSION_TOKEN").ok();
        let region = env::var("AWS_REGION")
            .or_else(|_| env::var("AWS_DEFAULT_REGION"))
            .unwrap_or_else(|_| config.region.clone());
            
        Ok(AwsCredentials {
            access_key,
            secret_key,
            session_token,
            expiration: None,
            region,
        })
    }
    
    /// Load credentials from AWS credentials file
    async fn load_file_credentials(config: &BedrockConfig) -> Result<AwsCredentials> {
        let home_dir = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| anyhow!("Cannot determine home directory"))?;
            
        let credentials_path = PathBuf::from(home_dir)
            .join(".aws")
            .join("credentials");
            
        let profile_name = config.profile.as_deref().unwrap_or("default");
        
        let content = fs::read_to_string(&credentials_path)
            .map_err(|e| anyhow!("Failed to read AWS credentials file: {}", e))?;
            
        let mut current_profile = None;
        let mut access_key = None;
        let mut secret_key = None;
        let mut session_token = None;
        
        for line in content.lines() {
            let line = line.trim();
            
            if line.starts_with('[') && line.ends_with(']') {
                current_profile = Some(line[1..line.len()-1].to_string());
            } else if let Some(profile) = &current_profile {
                if profile == profile_name {
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim();
                        
                        match key {
                            "aws_access_key_id" => access_key = Some(value.to_string()),
                            "aws_secret_access_key" => secret_key = Some(value.to_string()),
                            "aws_session_token" => session_token = Some(value.to_string()),
                            _ => {}
                        }
                    }
                }
            }
        }
        
        let access_key = access_key
            .ok_or_else(|| anyhow!("Access key not found in profile {}", profile_name))?;
        let secret_key = secret_key
            .ok_or_else(|| anyhow!("Secret key not found in profile {}", profile_name))?;
            
        Ok(AwsCredentials {
            access_key,
            secret_key,
            session_token,
            expiration: None,
            region: config.region.clone(),
        })
    }
    
    /// Load credentials from EC2 instance profile
    async fn load_instance_profile_credentials(config: &BedrockConfig) -> Result<AwsCredentials> {
        let client = reqwest::Client::new();
        
        // Get the role name
        let role_response = client
            .get("http://169.254.169.254/latest/meta-data/iam/security-credentials/")
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get instance profile role: {}", e))?;
            
        let role_name = role_response.text().await
            .map_err(|e| anyhow!("Failed to read role name: {}", e))?;
            
        // Get the credentials
        let cred_response = client
            .get(&format!("http://169.254.169.254/latest/meta-data/iam/security-credentials/{}", role_name))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get instance profile credentials: {}", e))?;
            
        let cred_json = cred_response.text().await
            .map_err(|e| anyhow!("Failed to read credentials JSON: {}", e))?;
            
        let ec2_creds: Ec2Credentials = serde_json::from_str(&cred_json)
            .map_err(|e| anyhow!("Failed to parse EC2 credentials: {}", e))?;
            
        let expiration = chrono::DateTime::parse_from_rfc3339(&ec2_creds.expiration)
            .map(|dt| dt.with_timezone(&Utc))
            .ok();
            
        Ok(AwsCredentials {
            access_key: ec2_creds.access_key_id,
            secret_key: ec2_creds.secret_access_key,
            session_token: ec2_creds.token,
            expiration,
            region: config.region.clone(),
        })
    }
    
    /// Load credentials from ECS task role
    async fn load_ecs_task_credentials(config: &BedrockConfig) -> Result<AwsCredentials> {
        let credentials_uri = env::var("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI")
            .map_err(|_| anyhow!("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI not found"))?;
            
        let client = reqwest::Client::new();
        let full_uri = format!("http://169.254.170.2{}", credentials_uri);
        
        let response = client
            .get(&full_uri)
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| anyhow!("Failed to get ECS task credentials: {}", e))?;
            
        let cred_json = response.text().await
            .map_err(|e| anyhow!("Failed to read ECS credentials JSON: {}", e))?;
            
        let ec2_creds: Ec2Credentials = serde_json::from_str(&cred_json)
            .map_err(|e| anyhow!("Failed to parse ECS credentials: {}", e))?;
            
        let expiration = chrono::DateTime::parse_from_rfc3339(&ec2_creds.expiration)
            .map(|dt| dt.with_timezone(&Utc))
            .ok();
            
        Ok(AwsCredentials {
            access_key: ec2_creds.access_key_id,
            secret_key: ec2_creds.secret_access_key,
            session_token: ec2_creds.token,
            expiration,
            region: config.region.clone(),
        })
    }
    
    /// Load credentials from Lambda environment
    async fn load_lambda_credentials(config: &BedrockConfig) -> Result<AwsCredentials> {
        let access_key = env::var("AWS_ACCESS_KEY_ID")
            .map_err(|_| anyhow!("AWS_ACCESS_KEY_ID not found in Lambda environment"))?;
            
        let secret_key = env::var("AWS_SECRET_ACCESS_KEY")
            .map_err(|_| anyhow!("AWS_SECRET_ACCESS_KEY not found in Lambda environment"))?;
            
        let session_token = env::var("AWS_SESSION_TOKEN").ok();
        
        Ok(AwsCredentials {
            access_key,
            secret_key,
            session_token,
            expiration: None,
            region: config.region.clone(),
        })
    }
    
    /// Load credentials using AWS default credential chain
    async fn load_default_credentials(config: &BedrockConfig) -> Result<AwsCredentials> {
        // Try in order: environment variables, credentials file, instance profile
        
        // 1. Try environment variables
        if let Ok(creds) = Self::load_env_credentials(config) {
            debug!("Using environment variable credentials");
            return Ok(creds);
        }
        
        // 2. Try credentials file
        if let Ok(creds) = Self::load_file_credentials(config).await {
            debug!("Using credentials file");
            return Ok(creds);
        }
        
        // 3. Try instance profile
        if let Ok(creds) = Self::load_instance_profile_credentials(config).await {
            debug!("Using instance profile credentials");
            return Ok(creds);
        }
        
        // 4. Try ECS task role
        if let Ok(creds) = Self::load_ecs_task_credentials(config).await {
            debug!("Using ECS task role credentials");
            return Ok(creds);
        }
        
        Err(anyhow!("No valid AWS credentials found"))
    }
    
    /// Sign a request using AWS SigV4
    pub fn sign_request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<HashMap<String, String>> {
        let now = Utc::now();
        let mut signed_headers = headers.clone();
        
        // Add required headers
        signed_headers.insert("host".to_string(), self.extract_host(url)?);
        signed_headers.insert("x-amz-date".to_string(), now.format("%Y%m%dT%H%M%SZ").to_string());
        
        if let Some(token) = &self.credentials.session_token {
            signed_headers.insert("x-amz-security-token".to_string(), token.clone());
        }
        
        // Create canonical request
        let canonical_request = self.create_canonical_request(
            method,
            url,
            &signed_headers,
            body,
        )?;
        
        // Create string to sign
        let string_to_sign = self.create_string_to_sign(&canonical_request, &now)?;
        
        // Calculate signature
        let signature = self.calculate_signature(&string_to_sign, &now)?;
        
        // Create authorization header
        let authorization = self.create_authorization_header(&signed_headers, &signature, &now)?;
        signed_headers.insert("Authorization".to_string(), authorization);
        
        Ok(signed_headers)
    }
    
    /// Extract host from URL
    fn extract_host(&self, url: &str) -> Result<String> {
        let parsed = url::Url::parse(url)
            .map_err(|e| anyhow!("Invalid URL: {}", e))?;
            
        Ok(parsed.host_str()
            .ok_or_else(|| anyhow!("No host in URL"))?
            .to_string())
    }
    
    /// Create canonical request for SigV4
    fn create_canonical_request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> Result<String> {
        let parsed = url::Url::parse(url)
            .map_err(|e| anyhow!("Invalid URL: {}", e))?;
            
        // Canonical URI
        let canonical_uri = if parsed.path().is_empty() { "/" } else { parsed.path() };
        
        // Canonical query string
        let canonical_query = parsed.query().unwrap_or("");
        
        // Canonical headers
        let mut header_names: Vec<String> = headers.keys().cloned().collect();
        header_names.sort();
        
        let canonical_headers: String = header_names
            .iter()
            .map(|name| {
                let value = headers.get(name).unwrap();
                format!("{}:{}\n", name.to_lowercase(), value.trim())
            })
            .collect();
            
        let signed_headers = header_names
            .iter()
            .map(|name| name.to_lowercase())
            .collect::<Vec<_>>()
            .join(";");
            
        // Payload hash
        let payload_hash = self.sha256_hex(body);
        
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method,
            canonical_uri,
            canonical_query,
            canonical_headers,
            signed_headers,
            payload_hash
        );
        
        trace!("Canonical request: {}", canonical_request);
        Ok(canonical_request)
    }
    
    /// Create string to sign for SigV4
    fn create_string_to_sign(&self, canonical_request: &str, now: &DateTime<Utc>) -> Result<String> {
        let date_stamp = now.format("%Y%m%d").to_string();
        let credential_scope = format!(
            "{}/{}/{}/aws4_request",
            date_stamp,
            self.credentials.region,
            self.service
        );
        
        let hashed_canonical_request = self.sha256_hex(canonical_request.as_bytes());
        
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            now.format("%Y%m%dT%H%M%SZ"),
            credential_scope,
            hashed_canonical_request
        );
        
        trace!("String to sign: {}", string_to_sign);
        Ok(string_to_sign)
    }
    
    /// Calculate AWS SigV4 signature
    fn calculate_signature(&self, string_to_sign: &str, now: &DateTime<Utc>) -> Result<String> {
        let date_stamp = now.format("%Y%m%d").to_string();
        
        let k_date = self.hmac_sha256(
            format!("AWS4{}", self.credentials.secret_key).as_bytes(),
            date_stamp.as_bytes(),
        );
        
        let k_region = self.hmac_sha256(&k_date, self.credentials.region.as_bytes());
        let k_service = self.hmac_sha256(&k_region, self.service.as_bytes());
        let k_signing = self.hmac_sha256(&k_service, b"aws4_request");
        
        let signature = self.hmac_sha256(&k_signing, string_to_sign.as_bytes());
        
        Ok(hex::encode(signature))
    }
    
    /// Create authorization header
    fn create_authorization_header(
        &self,
        headers: &HashMap<String, String>,
        signature: &str,
        now: &DateTime<Utc>,
    ) -> Result<String> {
        let date_stamp = now.format("%Y%m%d").to_string();
        let credential_scope = format!(
            "{}/{}/{}/aws4_request",
            date_stamp,
            self.credentials.region,
            self.service
        );
        
        let credential = format!("{}/{}", self.credentials.access_key, credential_scope);
        
        let mut header_names: Vec<String> = headers.keys().cloned().collect();
        header_names.sort();
        let signed_headers = header_names
            .iter()
            .map(|name| name.to_lowercase())
            .collect::<Vec<_>>()
            .join(";");
            
        Ok(format!(
            "AWS4-HMAC-SHA256 Credential={}, SignedHeaders={}, Signature={}",
            credential,
            signed_headers,
            signature
        ))
    }
    
    /// Calculate SHA256 hash and return as hex string
    fn sha256_hex(&self, data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
    
    /// Calculate HMAC-SHA256
    fn hmac_sha256(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        
        let mut mac = Hmac::<Sha256>::new_from_slice(key)
            .expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }
    
    /// Check if credentials are expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiration) = &self.credentials.expiration {
            Utc::now() > *expiration
        } else {
            false
        }
    }
    
    /// Refresh credentials if they are temporary and expired
    pub async fn refresh_if_needed(&mut self, config: &BedrockConfig) -> Result<()> {
        if self.is_expired() {
            warn!("Credentials expired, refreshing...");
            self.credentials = Self::load_credentials(config).await?;
        }
        Ok(())
    }
    
    /// Get the current credentials
    pub fn get_credentials(&self) -> &AwsCredentials {
        &self.credentials
    }
    
    /// Test credentials by making a minimal STS call
    pub async fn test_credentials(&self) -> Result<bool> {
        let client = reqwest::Client::new();
        let url = format!("https://sts.{}.amazonaws.com/", self.credentials.region);
        
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/x-amz-json-1.0".to_string());
        headers.insert("X-Amz-Target".to_string(), "AWSSecurityTokenServiceV20110615.GetCallerIdentity".to_string());
        
        let signed_headers = self.sign_request("POST", &url, &headers, b"{}")?;
        
        let mut request_headers = reqwest::header::HeaderMap::new();
        for (key, value) in signed_headers {
            if let (Ok(header_name), Ok(header_value)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value)
            ) {
                request_headers.insert(header_name, header_value);
            }
        }
        
        let response = client
            .post(&url)
            .headers(request_headers)
            .body("{}")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await?;
            
        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::aws_bedrock::types::BedrockConfig;

    #[test]
    fn test_sha256_hex() {
        let config = BedrockConfig::default();
        let credentials = AwsCredentials {
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            session_token: None,
            expiration: None,
            region: "us-east-1".to_string(),
        };
        
        let auth = BedrockAuth {
            credentials,
            auth_method: BedrockAuthMethod::Explicit,
            service: "bedrock".to_string(),
        };
        
        let result = auth.sha256_hex(b"test");
        assert_eq!(result, "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08");
    }

    #[test]
    fn test_extract_host() {
        let config = BedrockConfig::default();
        let credentials = AwsCredentials {
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            session_token: None,
            expiration: None,
            region: "us-east-1".to_string(),
        };
        
        let auth = BedrockAuth {
            credentials,
            auth_method: BedrockAuthMethod::Explicit,
            service: "bedrock".to_string(),
        };
        
        let host = auth.extract_host("https://bedrock-runtime.us-east-1.amazonaws.com/model/test/invoke").unwrap();
        assert_eq!(host, "bedrock-runtime.us-east-1.amazonaws.com");
    }

    #[test]
    fn test_credential_expiration() {
        let credentials = AwsCredentials {
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            session_token: None,
            expiration: Some(Utc::now() - chrono::Duration::hours(1)), // Expired 1 hour ago
            region: "us-east-1".to_string(),
        };
        
        let auth = BedrockAuth {
            credentials,
            auth_method: BedrockAuthMethod::Explicit,
            service: "bedrock".to_string(),
        };
        
        assert!(auth.is_expired());
    }

    #[test]
    fn test_credential_not_expired() {
        let credentials = AwsCredentials {
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            session_token: None,
            expiration: Some(Utc::now() + chrono::Duration::hours(1)), // Expires in 1 hour
            region: "us-east-1".to_string(),
        };
        
        let auth = BedrockAuth {
            credentials,
            auth_method: BedrockAuthMethod::Explicit,
            service: "bedrock".to_string(),
        };
        
        assert!(!auth.is_expired());
    }

    #[test]
    fn test_credential_no_expiration() {
        let credentials = AwsCredentials {
            access_key: "test".to_string(),
            secret_key: "test".to_string(),
            session_token: None,
            expiration: None,
            region: "us-east-1".to_string(),
        };
        
        let auth = BedrockAuth {
            credentials,
            auth_method: BedrockAuthMethod::Explicit,
            service: "bedrock".to_string(),
        };
        
        assert!(!auth.is_expired());
    }
}