use anyhow::anyhow;
use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::NaiveDateTime;
use fancy_regex::Regex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, sqlx::FromRow, Serialize, Deserialize, actix_jwt_auth_middleware::FromRequest,
)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_superuser: bool,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl User {
    pub fn new(
        username: &str,
        email: &str,
        password: &str,
        is_superuser: bool,
    ) -> anyhow::Result<Self> {
        if !validate_username(username)? {
            return Err(anyhow!(
                "Username must be at least 3 characters and contain only letters, numbers, or underscores."
            ));
        }

        if !validate_email(email)? {
            return Err(anyhow!("Invalid email address."));
        }

        if !validate_password(password)? {
            return Err(anyhow!(
                "Password must be at least 8 characters long and include at least one lowercase letter, one uppercase letter, and one number."
            ));
        }

        // Hash the password
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {e}"))?
            .to_string();

        Ok(User {
            id: 0, //set by DB
            username: username.to_string(),
            email: email.to_string(),
            password_hash,
            is_superuser,
            created_at: None, //set by DB
            updated_at: None, //set by DB
        })
    }

    pub fn verify_password(&self, password: &str) -> anyhow::Result<()> {
        let hash = PasswordHash::new(&self.password_hash)
            .map_err(|e| anyhow!("Failed to generate passped hash: {}", e))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .map_err(|e| anyhow!("Password not match: {}", e))
    }
}

fn validate_username(username: &str) -> anyhow::Result<bool> {
    static RE: Lazy<Option<Regex>> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9_]{3,}$").ok());
    match &*RE {
        Some(re) => re
            .is_match(username)
            .map_err(|e| anyhow!("Regex error for username: {e}")),
        None => Err(anyhow!(
            "Username regex failed to compile. Rejecting all usernames."
        )),
    }
}

fn validate_email(email: &str) -> anyhow::Result<bool> {
    static RE: Lazy<Option<Regex>> = Lazy::new(|| Regex::new(r"^[^@]+@[^@]+\.[^@]+$").ok());
    match &*RE {
        Some(re) => re
            .is_match(email)
            .map_err(|e| anyhow!("Regex error for email: {e}")),
        None => Err(anyhow!(
            "Email regex failed to compile. Rejecting all emails."
        )),
    }
}

fn validate_password(password: &str) -> anyhow::Result<bool> {
    static RE: Lazy<Option<Regex>> =
        Lazy::new(|| Regex::new(r"^(?=.*[a-z])(?=.*[A-Z])(?=.*\d).{8,}$").ok());
    match &*RE {
        Some(re) => re
            .is_match(password)
            .map_err(|e| anyhow!("Regex error for password: {e}")),
        None => Err(anyhow!(
            "Password regex failed to compile. Rejecting all passwords."
        )),
    }
}
