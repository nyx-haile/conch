use chrono::{DateTime, Utc};

pub const LEGAL_VERSION: &str = "2026-04-26";
pub const REQUIRED_BILLING_DISCLOSURE: &str =
    "BYOK or usage-based managed billing. No subscription. No automatic charge.";
pub const RECORDING_CONSENT_BUTTON: &str = "I have recording consent — start voice session";

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
    let trial_href = ctas.trial_signup_url.as_deref().unwrap_or("/app/signup");

    format!(
        r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Conch — Voice interviews to launch-ready briefs</title>
  <meta name="description" content="Open Conch, start a trial, and turn spoken discovery into structured briefs." />
  <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
</head>
<body>
  <header>
    <strong>Conch</strong>
    <nav>
      <a href="/app">Open app</a>
      <a href="/app/signup">Start free</a>
      <a href="/terms.html">Terms</a>
      <a href="/privacy.html">Privacy</a>
      <a href="/recording-consent.html">Recording Consent</a>
    </nav>
  </header>
  <main>
    <section>
      <p>Voice interviews for builders</p>
      <h1>Open Conch and start talking.</h1>
      <p>Run a live voice interview, capture the messy thinking, and leave with a structured brief your team can act on.</p>
      <p><strong>{billing_disclosure}</strong> This beta app does not collect payment card data in the public shell. Audio is processed only after a session starts in the app and you accept recording consent.</p>
      <p>
        <a href="{trial_href}">Start free</a>
        <a href="/app">Open Conch</a>
      </p>
    </section>
    <section id="usage">
      <h2>BYOK or usage-based</h2>
      <ul>
        <li>Free trial starts in the app.</li>
        <li>Bring provider/model keys where Conch supports it.</li>
        <li>Use Conch-managed Deepgram and model usage on a metered basis when enabled.</li>
      </ul>
      <p>Usage is based on actual managed voice/model activity. Local/offline sessions do not consume managed credits.</p>
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
    <a href="mailto:{support_email}">Support</a>
  </footer>
</body>
</html>"##,
        billing_disclosure = REQUIRED_BILLING_DISCLOSURE,
        billing_email = config.billing_email,
        status_url = config.status_url,
        support_email = config.support_email,
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
    <p>{billing_disclosure} Conch charges only after a separate explicit purchase or managed-usage agreement. The public app shell does not collect payment card data.</p>
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
    <p>Conch collects the minimum account, session, consent, audio, transcript, generated brief, provider-routing metadata, and logs needed to run the service.</p>
    <p>Deepgram may process audio for cloud speech-to-text and voice features when cloud mode is enabled. Payment processors may process billing metadata only when a separate paid workflow is enabled; raw card data does not touch Conch servers.</p>
    <p>Provider API keys are server-only and are not exposed to browser code.</p>
    <p>The public app requests microphone access only after recording consent and only when the Deepgram token path is configured.</p>
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
    <p>The app must show a consent gate before any microphone capture or Deepgram streaming.</p>
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
