//! Auth/onboarding surface for the Conch web launch.
//!
//! The beta account model is deliberately small: one owner user, one
//! workspace, required legal acceptance, then Stripe setup-mode card
//! verification before trial credits are provisioned.

use crate::billing::{trial_setup_checkout, StripeCheckoutSpec, BILLING_DISCLOSURE};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CURRENT_TERMS_VERSION: &str = "terms-2026-04-25";
pub const CURRENT_PRIVACY_VERSION: &str = "privacy-2026-04-25";
pub const CURRENT_RECORDING_CONSENT_VERSION: &str = "recording-consent-2026-04-25";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    PendingCardSetup,
    Active,
    Suspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceTier {
    Trial,
    Free,
    Paid,
    Pilot,
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignupRequest {
    pub email: String,
    pub accepted_terms_version: Option<String>,
    pub accepted_privacy_version: Option<String>,
    pub accepted_recording_consent_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountOnboarding {
    pub user_id: String,
    pub workspace_id: String,
    pub email: String,
    pub status: AccountStatus,
    pub tier: WorkspaceTier,
    pub stripe_customer_id: String,
    pub terms_version: String,
    pub privacy_version: String,
    pub recording_consent_version: String,
    pub accepted_at: DateTime<Utc>,
    pub billing_disclosure: &'static str,
}

impl AccountOnboarding {
    pub fn checkout_for_card_gate(
        &self,
        success_url: impl Into<String>,
        cancel_url: impl Into<String>,
    ) -> StripeCheckoutSpec {
        trial_setup_checkout(
            self.stripe_customer_id.clone(),
            self.workspace_id.clone(),
            success_url,
            cancel_url,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthSurfaceCopy {
    pub headline: &'static str,
    pub primary_cta: &'static str,
    pub billing_disclosure: &'static str,
    pub legal_links: Vec<LegalLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalLink {
    pub label: &'static str,
    pub href: &'static str,
}

pub fn auth_surface_copy() -> AuthSurfaceCopy {
    AuthSurfaceCopy {
        headline: "Start Conch with a card-verified free trial",
        primary_cta: "Start free — card required",
        billing_disclosure: BILLING_DISCLOSURE,
        legal_links: vec![
            LegalLink {
                label: "Terms",
                href: "/terms.html",
            },
            LegalLink {
                label: "Privacy",
                href: "/privacy.html",
            },
            LegalLink {
                label: "Recording Consent",
                href: "/recording-consent.html",
            },
        ],
    }
}

pub fn start_signup(
    request: SignupRequest,
    user_id: impl Into<String>,
    workspace_id: impl Into<String>,
    stripe_customer_id: impl Into<String>,
    now: DateTime<Utc>,
) -> Result<AccountOnboarding, AuthError> {
    validate_email(&request.email)?;
    require_version(
        "terms",
        request.accepted_terms_version.as_deref(),
        CURRENT_TERMS_VERSION,
    )?;
    require_version(
        "privacy",
        request.accepted_privacy_version.as_deref(),
        CURRENT_PRIVACY_VERSION,
    )?;
    require_version(
        "recording consent",
        request.accepted_recording_consent_version.as_deref(),
        CURRENT_RECORDING_CONSENT_VERSION,
    )?;

    Ok(AccountOnboarding {
        user_id: user_id.into(),
        workspace_id: workspace_id.into(),
        email: request.email.trim().to_ascii_lowercase(),
        status: AccountStatus::PendingCardSetup,
        tier: WorkspaceTier::Trial,
        stripe_customer_id: stripe_customer_id.into(),
        terms_version: CURRENT_TERMS_VERSION.to_owned(),
        privacy_version: CURRENT_PRIVACY_VERSION.to_owned(),
        recording_consent_version: CURRENT_RECORDING_CONSENT_VERSION.to_owned(),
        accepted_at: now,
        billing_disclosure: BILLING_DISCLOSURE,
    })
}

fn validate_email(email: &str) -> Result<(), AuthError> {
    let trimmed = email.trim();
    let has_single_at = trimmed.matches('@').count() == 1;
    let has_domain_dot = trimmed
        .split('@')
        .nth(1)
        .is_some_and(|domain| domain.contains('.'));
    if has_single_at && has_domain_dot {
        Ok(())
    } else {
        Err(AuthError::InvalidEmail)
    }
}

fn require_version(
    label: &'static str,
    provided: Option<&str>,
    expected: &'static str,
) -> Result<(), AuthError> {
    match provided {
        Some(version) if version == expected => Ok(()),
        Some(_) => Err(AuthError::OutdatedLegalAcceptance { label, expected }),
        None => Err(AuthError::MissingLegalAcceptance { label, expected }),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    #[error("email is invalid")]
    InvalidEmail,
    #[error("missing {label} acceptance; expected {expected}")]
    MissingLegalAcceptance {
        label: &'static str,
        expected: &'static str,
    },
    #[error("outdated {label} acceptance; expected {expected}")]
    OutdatedLegalAcceptance {
        label: &'static str,
        expected: &'static str,
    },
}
