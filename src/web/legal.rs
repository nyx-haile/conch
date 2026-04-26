use chrono::{DateTime, Utc};

pub const LEGAL_VERSION: &str = "2026-04-26";
pub const REQUIRED_BILLING_DISCLOSURE: &str =
    "Card required. No subscription. No automatic charge.";
pub const RECORDING_CONSENT_BUTTON: &str = "I have consent — start recording";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicSiteConfig {
    pub company_name: String,
    pub company_legal_name: String,
    pub company_address: String,
    pub support_email: String,
    pub billing_email: String,
    pub status_url: String,
}

impl PublicSiteConfig {
    pub fn beta_placeholder() -> Self {
        Self {
            company_name: "Conch".to_string(),
            company_legal_name: "Conch beta".to_string(),
            company_address: "United States".to_string(),
            support_email: "conch@theos.sh".to_string(),
            billing_email: "conch@theos.sh".to_string(),
            status_url: "/status.html".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtaLinks {
    pub trial_signup_url: Option<String>,
    pub starter_pack_url: Option<String>,
    pub team_pack_url: Option<String>,
    pub pilot_pack_url: Option<String>,
}

impl CtaLinks {
    pub fn disabled() -> Self {
        Self {
            trial_signup_url: None,
            starter_pack_url: None,
            team_pack_url: None,
            pilot_pack_url: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingConsentAcceptance {
    pub workspace_id: String,
    pub version: String,
    pub accepted_at: DateTime<Utc>,
}

impl RecordingConsentAcceptance {
    pub fn new(workspace_id: impl Into<String>, accepted_at: DateTime<Utc>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            version: LEGAL_VERSION.to_string(),
            accepted_at,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterviewStartGate {
    Allowed,
    MissingRecordingConsent,
}

pub fn require_recording_consent(
    consent: Option<&RecordingConsentAcceptance>,
) -> InterviewStartGate {
    if consent.is_some() {
        InterviewStartGate::Allowed
    } else {
        InterviewStartGate::MissingRecordingConsent
    }
}

pub fn checkout_disclosure() -> &'static str {
    REQUIRED_BILLING_DISCLOSURE
}

pub fn landing_page(config: &PublicSiteConfig, ctas: &CtaLinks) -> String {
    let trial_href = ctas
        .trial_signup_url
        .as_deref()
        .unwrap_or("#payments-disabled");
    let starter_href = ctas
        .starter_pack_url
        .as_deref()
        .unwrap_or("#payments-disabled");
    let team_href = ctas
        .team_pack_url
        .as_deref()
        .unwrap_or("#payments-disabled");
    let pilot_href = ctas
        .pilot_pack_url
        .as_deref()
        .unwrap_or("#payments-disabled");
    let disabled_attr = if ctas.trial_signup_url.is_none() {
        r#" aria-disabled="true""#
    } else {
        ""
    };

    format!(
        r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Conch — Voice interviews to launch-ready briefs</title>
  <meta name="description" content="Run guided voice interviews and turn messy spoken discovery into structured briefs." />
  <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
</head>
<body>
  <header>
    <strong>Conch</strong>
    <nav>
      <a href="/terms.html">Terms</a>
      <a href="/privacy.html">Privacy</a>
      <a href="/recording-consent.html">Recording Consent</a>
      <a href="mailto:{support_email}">Contact</a>
    </nav>
  </header>
  <main>
    <section>
      <p>Voice interviews for builders</p>
      <h1>Conch turns spoken discovery into launch-ready briefs.</h1>
      <p>Run a live voice interview, capture the messy thinking, and leave with a structured brief your team can act on.</p>
      <p><strong>{billing_disclosure}</strong> This beta contact surface does not collect microphone audio, payment card data, or provider API keys. Audio is processed only after a session starts in the app and you are responsible for getting consent before recording other people.</p>
      <p>
        <a href="{trial_href}"{disabled_attr}>Start free — card required, no auto-charge</a>
        <a href="#pricing">Buy prepaid usage credits</a>
      </p>
    </section>
    <section id="pricing">
      <h2>Pricing</h2>
      <ul>
        <li>Free trial — $0, 120 STT minutes, 14 days, card required, no auto-charge</li>
        <li><a href="{starter_href}">Starter</a> — $29 one-time, 1,000 STT minutes</li>
        <li><a href="{team_href}">Team</a> — $199 one-time, 10,000 STT minutes</li>
        <li><a href="{pilot_href}">Pilot</a> — $499 one-time, 30,000 STT minutes plus onboarding</li>
      </ul>
      <p>Usage credits only count cloud speech-to-text minutes processed through Conch. Local/offline sessions do not consume paid credits.</p>
    </section>
    <section>
      <h2>Consent and privacy</h2>
      <p>Conch can record and transcribe audio. Laws vary by location. If a session includes anyone besides you, get clear permission from every participant before recording.</p>
    </section>
  </main>
  <footer>
    <a href="/terms.html">Terms of Service</a>
    <a href="/privacy.html">Privacy Policy</a>
    <a href="/recording-consent.html">Recording Consent Notice</a>
    <a href="{status_url}">Status</a>
    <a href="mailto:{billing_email}">Billing</a>
  </footer>
</body>
</html>"##,
        billing_disclosure = REQUIRED_BILLING_DISCLOSURE,
        billing_email = config.billing_email,
        disabled_attr = disabled_attr,
        pilot_href = pilot_href,
        starter_href = starter_href,
        status_url = config.status_url,
        support_email = config.support_email,
        team_href = team_href,
        trial_href = trial_href,
    )
}

pub fn terms_page(config: &PublicSiteConfig) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8" /><meta name="viewport" content="width=device-width, initial-scale=1" /><title>Conch Terms of Service</title><link rel="icon" href="/favicon.svg" type="image/svg+xml" /></head>
<body>
  <main>
    <h1>Terms of Service</h1>
    <p>Version {version}</p>
    <p>{company_legal_name}, at {company_address}, provides {company_name} for beta voice interview sessions.</p>
    <h2>Payments</h2>
    <p>{billing_disclosure} A saved payment method from the free trial is a card-verification signal only. Conch charges you only when you separately buy prepaid usage credits. This static deployment is an information and contact surface, not an active payment checkout.</p>
    <h2>Recording responsibilities</h2>
    <p>You are responsible for obtaining every consent required by law before recording or transcribing anyone. Do not use Conch for unlawful surveillance, biometric identification, children under 13, or regulated workflows without written approval.</p>
    <h2>Data rights</h2>
    <p>You may request export, deletion, saved-card removal, or billing help at <a href="mailto:{support_email}">{support_email}</a>.</p>
  </main>
</body>
</html>"#,
        billing_disclosure = REQUIRED_BILLING_DISCLOSURE,
        company_address = config.company_address,
        company_legal_name = config.company_legal_name,
        company_name = config.company_name,
        support_email = config.support_email,
        version = LEGAL_VERSION,
    )
}

pub fn privacy_page(config: &PublicSiteConfig) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8" /><meta name="viewport" content="width=device-width, initial-scale=1" /><title>Conch Privacy Policy</title><link rel="icon" href="/favicon.svg" type="image/svg+xml" /></head>
<body>
  <main>
    <h1>Privacy Policy</h1>
    <p>Version {version}</p>
    <p>Conch collects the minimum account, Stripe customer/payment metadata, audio, transcript, generated brief, provider-routing metadata, and logs needed to run the service.</p>
    <p>Deepgram may process audio for cloud speech-to-text when cloud mode is enabled. Stripe processes payments and stores card details; raw card data does not touch Conch servers.</p>
    <p>Provider API keys are server-only and are not exposed to browser code.</p>
    <p>This static deployment does not collect microphone audio, payment card data, or provider API keys.</p>
    <p>Contact <a href="mailto:{support_email}">{support_email}</a> for export, deletion, or saved-card removal requests.</p>
  </main>
</body>
</html>"#,
        support_email = config.support_email,
        version = LEGAL_VERSION,
    )
}

pub fn recording_consent_page(config: &PublicSiteConfig) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8" /><meta name="viewport" content="width=device-width, initial-scale=1" /><title>Conch Recording Consent Notice</title><link rel="icon" href="/favicon.svg" type="image/svg+xml" /></head>
<body>
  <main>
    <h1>Recording Consent Notice</h1>
    <p>Version {version}</p>
    <p>Conch records and transcribes audio when you start a session. Recording laws vary by location. Some places require consent from every participant.</p>
    <p>By starting a session, you confirm that you have the right to record and transcribe the conversation and that every required participant has consented.</p>
    <p>This static launch page does not request microphone access. The app must show a consent gate before any microphone capture or Deepgram streaming.</p>
    <p>In-app consent action label: <strong>{button}</strong>.</p>
    <p>Questions? Contact <a href="mailto:{support_email}">{support_email}</a>.</p>
  </main>
</body>
</html>"#,
        button = RECORDING_CONSENT_BUTTON,
        support_email = config.support_email,
        version = LEGAL_VERSION,
    )
}
