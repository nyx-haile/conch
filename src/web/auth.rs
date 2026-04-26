use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignupRequest {
    pub email: String,
    pub accepted_terms_version: String,
}

impl SignupRequest {
    pub fn new(
        email: impl Into<String>,
        accepted_terms_version: impl Into<String>,
    ) -> Result<Self, AuthError> {
        let email = normalize_email(email.into())?;
        let accepted_terms_version = accepted_terms_version.into();
        if accepted_terms_version.trim().is_empty() {
            return Err(AuthError::MissingTermsAcceptance);
        }
        Ok(Self {
            email,
            accepted_terms_version,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePrincipal {
    pub user_id: String,
    pub workspace_id: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthSession {
    pub principal: WorkspacePrincipal,
    pub expires_at: DateTime<Utc>,
}

impl AuthSession {
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.expires_at > now
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthError {
    EmptyEmail,
    InvalidEmail,
    MissingTermsAcceptance,
    MissingSession,
    ExpiredSession,
}

pub fn require_active_session<'a>(
    session: Option<&'a AuthSession>,
    now: DateTime<Utc>,
) -> Result<&'a WorkspacePrincipal, AuthError> {
    let session = session.ok_or(AuthError::MissingSession)?;
    if !session.is_active_at(now) {
        return Err(AuthError::ExpiredSession);
    }
    Ok(&session.principal)
}

fn normalize_email(email: String) -> Result<String, AuthError> {
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() {
        return Err(AuthError::EmptyEmail);
    }
    let (local, domain) = email.split_once('@').ok_or(AuthError::InvalidEmail)?;
    if local.is_empty()
        || domain.is_empty()
        || domain.starts_with('.')
        || domain.ends_with('.')
        || !domain.contains('.')
    {
        return Err(AuthError::InvalidEmail);
    }
    Ok(email)
}
