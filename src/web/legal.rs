//! Static legal and trust-copy surfaces for the Conch web launch.
//!
//! These are product/legal placeholders for implementation and tests. They
//! must be reviewed by counsel and populated with the final company identity
//! before accepting public payments.

use crate::billing::{BILLING_DISCLOSURE, STRIPE_CARD_STORAGE_DISCLOSURE};
use serde::{Deserialize, Serialize};

pub const COMPANY_LEGAL_NAME_PLACEHOLDER: &str = "Conch Legal Entity TBD";
pub const SUPPORT_EMAIL_PLACEHOLDER: &str = "support@example.com";
pub const PRIVACY_EMAIL_PLACEHOLDER: &str = "privacy@example.com";
pub const BILLING_EMAIL_PLACEHOLDER: &str = "billing@example.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegalPageKind {
    Terms,
    Privacy,
    RecordingConsent,
    BillingPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalPage {
    pub kind: LegalPageKind,
    pub slug: &'static str,
    pub path: &'static str,
    pub title: &'static str,
    pub version: &'static str,
    pub body: &'static str,
}

pub fn legal_pages() -> Vec<LegalPage> {
    vec![terms_page(), privacy_page(), recording_consent_page(), billing_policy_page()]
}

pub fn legal_footer_links() -> Vec<(&'static str, &'static str)> {
    legal_pages()
        .into_iter()
        .map(|page| (page.title, page.path))
        .collect()
}

pub fn terms_page() -> LegalPage {
    LegalPage {
        kind: LegalPageKind::Terms,
        slug: "terms",
        path: "/terms.html",
        title: "Terms",
        version: "terms-2026-04-25",
        body: concat!(
            "Conch accounts are for users 18 and older. Users must have authority ",
            "to use Conch for a workspace. Users are responsible for obtaining ",
            "recording consent before capturing or transcribing anyone else's voice. ",
            "Prohibited use includes illegal recording, child-directed use, biometric ",
            "identification, malware or abuse, and regulated health, finance, legal, ",
            "or other high-risk workflows unless separately approved in writing. ",
            "Payment terms: ",
            "Card required for the free trial. No subscription. No automatic charge. ",
            "Stripe processes payments and stores any saved payment method; Conch ",
            "does not receive raw card data. A saved trial card may be charged only ",
            "when the user separately completes an explicit paid checkout or later ",
            "opts into another user-initiated paid top-up feature. Contact billing ",
            "for refunds of unused paid usage credits."
        ),
    }
}

pub fn privacy_page() -> LegalPage {
    LegalPage {
        kind: LegalPageKind::Privacy,
        slug: "privacy",
        path: "/privacy.html",
        title: "Privacy",
        version: "privacy-2026-04-25",
        body: concat!(
            "Conch collects account email, Stripe customer and payment-method ",
            "metadata, billing metadata, audio, transcripts, generated briefs, ",
            "provider configuration metadata, and security/support logs to provide ",
            "the interview, transcription, briefing, trial activation, billing, ",
            "support, security, and product analytics services. Processors may include ",
            "Deepgram, Stripe, a hosting provider, an email provider, and analytics ",
            "tools if configured. Provider API keys stay server-side and are not ",
            "exposed to the browser. Raw audio is not persisted by default for the MVP; ",
            "transcripts and briefs are retained until deletion/export handling. Users ",
            "can contact privacy@example.com for access, deletion, export, or card ",
            "metadata removal support."
        ),
    }
}

pub fn recording_consent_page() -> LegalPage {
    LegalPage {
        kind: LegalPageKind::RecordingConsent,
        slug: "recording-consent",
        path: "/recording-consent.html",
        title: "Recording Consent",
        version: "recording-consent-2026-04-25",
        body: concat!(
            "Conch can record and transcribe audio. Recording and consent laws vary ",
            "by location. Before using the microphone or streaming audio to Deepgram, ",
            "the user must confirm they have permission to record and transcribe all ",
            "participants. If anyone else is present, get clear consent from every ",
            "participant before starting. Do not use Conch with children under 13 or ",
            "for regulated workflows unless those requirements have been reviewed and ",
            "approved separately."
        ),
    }
}

pub fn billing_policy_page() -> LegalPage {
    LegalPage {
        kind: LegalPageKind::BillingPolicy,
        slug: "billing-policy",
        path: "/billing.html",
        title: "Billing Policy",
        version: "billing-2026-04-25",
        body: concat!(
            "The launch offer uses a card-verified free trial and one-time prepaid ",
            "usage packs. Card required for the free trial. No subscription. No ",
            "automatic charge. Starter, Team, and Pilot packs are one-time prepaid ",
            "purchases. Stripe stores payment methods; Conch never receives or stores ",
            "raw card data. Unused trial cards should be detached after the retention ",
            "window if no paid purchase occurs, unless support, fraud, chargeback, or ",
            "legal retention requires otherwise. Contact billing@example.com for ",
            "billing questions and unused paid-credit refund requests."
        ),
    }
}

pub fn payment_trust_copy() -> Vec<&'static str> {
    vec![
        BILLING_DISCLOSURE,
        "Prepaid packs are one-time purchases for Conch usage credits.",
        "No automatic renewal and no free-to-paid conversion.",
        STRIPE_CARD_STORAGE_DISCLOSURE,
        "Public payment links must stay disabled until company identity, support, privacy, and billing contacts are final.",
    ]
}
