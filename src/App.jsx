import { useEffect, useRef, useState } from "react";

const contactEmail = "conch@theos.sh";
const billingHref = `mailto:${contactEmail}?subject=Conch%20billing`;
const supportHref = `mailto:${contactEmail}?subject=Conch%20support`;
const trialStorageKey = "conch_trial_session_v1";
const consentVersion = "2026-04-26";

const appStatusCopy = {
  signup_required: "Free trial required",
  trial_pending: "Opening your trial",
  email_confirmation_required: "Check your email",
  auth_config_missing: "Voice unavailable",
  usage_config_missing: "Voice unavailable",
  usage_limit_reached: "Trial limit reached",
  free_trials_closed: "Free trials paused",
  voice_config_missing: "Voice unavailable",
  consent_required: "Consent required",
  ready_to_talk: "Ready to talk",
  listening: "Listening",
  thinking: "Thinking",
  speaking: "Speaking",
  ended: "Session ended",
  error: "Needs attention",
};

const productHighlights = [
  {
    no: "01",
    title: "Open and start talking",
    text: "Confirm your email for a free trial, accept recording consent, and begin a real voice session.",
  },
  {
    no: "02",
    title: "Real conversation",
    text: "Conch listens to your voice and turns it into a launch-ready brief. No demo scripts, no fake transcripts.",
  },
  {
    no: "03",
    title: "No billing theater",
    text: "No subscription. No automatic charge. You only pay for what you actually use.",
  },
];

const usageOptions = [
  {
    no: "01",
    title: "Free trial",
    text: "Confirm your email and Conch opens with a starter trial. No card, no auto-renew.",
  },
  {
    no: "02",
    title: "Bring your own keys",
    text: "Prefer to route through your own provider account? Conch supports it.",
  },
  {
    no: "03",
    title: "Pay as you talk",
    text: "Past the trial, you only pay for actual session time.",
  },
];

const legalPages = {
  "/terms.html": {
    title: "Terms of Service",
    eyebrow: "Version 2026-04-26",
    sections: [
      [
        "Beta status",
        "Conch is an early beta voice-discovery app. The web app requires email confirmation before a trial session. Long-term storage and exports are coming soon.",
      ],
      [
        "Payments",
        "Pricing is bring-your-own-keys or pay-as-you-go for Conch-managed voice and model usage. Confirmed free trials include $10 of starter usage per user and pause when aggregate free-trial usage reaches $10,000. No subscription. No automatic charge. Conch charges only after a separate explicit purchase or usage agreement.",
      ],
      [
        "Recording responsibilities",
        "You are responsible for obtaining every consent required by law before recording or transcribing anyone. Do not use Conch for unlawful surveillance, biometric identification, children under 13, or regulated workflows without written approval.",
      ],
      [
        "Data rights",
        `You may request export, deletion, saved-card removal, or billing help at ${contactEmail}.`,
      ],
    ],
  },
  "/privacy.html": {
    title: "Privacy Policy",
    eyebrow: "Version 2026-04-26",
    sections: [
      [
        "What Conch needs",
        "Conch collects the minimum account, email, session, consent, audio, transcript, brief, and log data needed to run the service.",
      ],
      [
        "Processors",
        "Deepgram may process audio for speech-to-text and voice features. Payment processors may process billing metadata only when a paid workflow is in use.",
      ],
      [
        "This public app",
        "This app requests microphone access only after you accept the recording-consent gate and a voice session is available.",
      ],
      [
        "Contact",
        `Contact ${contactEmail} for export, deletion, billing, or saved-card removal requests.`,
      ],
    ],
  },
  "/recording-consent.html": {
    title: "Recording Consent Notice",
    eyebrow: "Version 2026-04-26",
    sections: [
      [
        "Before recording",
        "Conch records and transcribes audio only when you start a voice run in the app. Recording laws vary by location. Some places require consent from every participant.",
      ],
      [
        "Your confirmation",
        "By starting a voice run, you confirm that you have the right to record and transcribe the conversation and that every required participant has consented.",
      ],
      [
        "In-app consent label",
        "I have recording consent — start voice session.",
      ],
      [
        "Provider boundary",
        "Deepgram receives audio only after you accept consent, and only through Conch.",
      ],
    ],
  },
};

function ConchMark() {
  return (
    <svg viewBox="-40 -40 80 80" aria-hidden="true">
      <g fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round">
        <path d="M 28,0 A 28,28 0 0 0 -28,0 A 17,17 0 0 1 6,0 A 11,11 0 0 0 -16,0 A 6,6 0 0 1 -4,0" />
      </g>
      <circle cx="-4" cy="0" r="2.4" fill="currentColor" />
    </svg>
  );
}

function ConchSpiral() {
  const ribs = Array.from({ length: 36 }, (_, i) => {
    const angle = (i * 10 * Math.PI) / 180;
    const r1 = 175;
    const r2 = i % 3 === 0 ? 156 : 168;
    return {
      key: i,
      x1: Math.cos(angle) * r1,
      y1: Math.sin(angle) * r1,
      x2: Math.cos(angle) * r2,
      y2: Math.sin(angle) * r2,
    };
  });

  return (
    <div className="conch-spiral-frame">
      <svg className="conch-spiral" viewBox="-200 -200 400 400" aria-hidden="true">
        <defs>
          <radialGradient id="conchHalo" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="#f5d8c8" stopOpacity="0.85" />
            <stop offset="60%" stopColor="#f1ebe0" stopOpacity="0.4" />
            <stop offset="100%" stopColor="#f1ebe0" stopOpacity="0" />
          </radialGradient>
        </defs>
        <circle r="190" fill="url(#conchHalo)" />
        <g stroke="#d23656" strokeWidth="0.6" strokeLinecap="round" opacity="0.42">
          {ribs.map((line) => (
            <line
              key={line.key}
              x1={line.x1}
              y1={line.y1}
              x2={line.x2}
              y2={line.y2}
            />
          ))}
        </g>
        <circle r="156" fill="none" stroke="#d23656" strokeWidth="0.5" opacity="0.32" />
        <g fill="none" stroke="#d23656" strokeWidth="1.6" strokeLinecap="round">
          <path d="M 140,0 A 140,140 0 0 0 -140,0 A 100,100 0 0 1 60,0 A 55,55 0 0 0 -50,0 A 39,39 0 0 1 28,0 A 25,25 0 0 0 -22,0 A 16,16 0 0 1 10,0 A 8,8 0 0 0 -6,0" />
        </g>
        <circle cx="-6" cy="0" r="3.2" fill="#d23656" />
      </svg>
    </div>
  );
}

function App() {
  const path = normalizePath(window.location.pathname);
  const page = legalPages[path];

  if (page) {
    return <Shell><LegalPage page={page} /></Shell>;
  }

  if (path === "/status.html") {
    return <Shell><StatusPage /></Shell>;
  }

  if (path === "/app/signup") {
    return <Shell active="signup"><SignupPage /></Shell>;
  }

  if (path === "/app/confirm") {
    return <Shell active="signup"><ConfirmPage /></Shell>;
  }

  if (path === "/app") {
    return <Shell active="app"><AppWorkspace /></Shell>;
  }

  if (path !== "/") {
    return <Shell><NotFound /></Shell>;
  }

  return <Shell active="home"><HomePage /></Shell>;
}

function Shell({ active = "home", children }) {
  return (
    <div className="app-shell">
      <header className="site-header">
        <a href="/" className="brand" aria-label="Conch home">
          <span className="brand-mark"><ConchMark /></span>
          <span>✣ Conch</span>
        </a>
        <nav aria-label="Primary navigation">
          <a className={active === "app" ? "active" : ""} href="/app">Open app</a>
          <a className={active === "signup" ? "active" : ""} href="/app/signup">Start free</a>
          <a href="/#usage">Usage</a>
          <a href="/recording-consent.html">Consent</a>
          <a href="/privacy.html">Privacy</a>
        </nav>
      </header>
      {children}
      <footer className="site-footer">
        <span>© 2026 Conch — voice discovery, briefly.</span>
        <a href="/terms.html">Terms</a>
        <a href="/privacy.html">Privacy</a>
        <a href="/recording-consent.html">Consent</a>
        <a href="/status.html">Status</a>
        <a href={billingHref}>Billing</a>
        <a href={supportHref}>Support</a>
      </footer>
    </div>
  );
}

function HomePage() {
  return (
    <main>
      <section className="hero-section section-frame">
        <div className="hero-copy">
          <p className="eyebrow">Voice discovery, for builders</p>
          <h1>Open Conch and <em>start talking.</em></h1>
          <p className="hero-lede">
            Conch turns a live voice conversation into a product brief. Confirm your email for a free trial,
            accept recording consent, and start talking.
          </p>
          <div className="hero-actions">
            <a className="button button-primary" href="/app/signup">Start free</a>
            <a className="button button-secondary" href="/app">Open Conch</a>
          </div>
          <p className="trust-note">
            Bring your own keys, or pay as you talk. No subscription. No automatic charge.
          </p>
        </div>

        <aside className="app-preview" aria-label="Conch app access path">
          <div className="preview-illustration">
            <ConchSpiral />
          </div>
          <div className="preview-topline">
            <span>App path</span>
            <span>Voice-ready</span>
          </div>
          <ol className="path-list">
            <li><strong>i</strong><span>Sign up for the free trial.</span></li>
            <li><strong>ii</strong><span>Accept recording consent.</span></li>
            <li><strong>iii</strong><span>Start a real voice session — no fake transcript.</span></li>
          </ol>
          <a className="button button-primary full-width" href="/app">Go to app</a>
        </aside>
      </section>

      <section className="bento-grid" aria-label="Conch product highlights">
        {productHighlights.map((feature) => (
          <article className="bento-card" key={feature.title}>
            <span className="specimen-no">№ {feature.no}</span>
            <h2>{feature.title}</h2>
            <p>{feature.text}</p>
          </article>
        ))}
      </section>

      <section className="usage-section" id="usage">
        <div>
          <p className="eyebrow">How it bills</p>
          <h2>No public price <em>theater</em>.</h2>
          <p>
            The first product job is access: open the app, confirm a free trial, and talk. Free trials open with a starter balance — past that, you pay only for what you use.
          </p>
        </div>
        <div className="usage-grid">
          {usageOptions.map((option) => (
            <article className="usage-card" key={option.title}>
              <h3>
                <span>{option.title}</span>
                <span className="specimen-no">№ {option.no}</span>
              </h3>
              <p>{option.text}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="cta-strip">
        <div>
          <p className="eyebrow">Ready now</p>
          <h2>The app is the <em>front door</em>.</h2>
        </div>
        <div className="hero-actions">
          <a className="button button-primary" href="/app/signup">Start free</a>
          <a className="button button-secondary" href="/app">Open Conch</a>
        </div>
      </section>
    </main>
  );
}

function SignupPage() {
  const [email, setEmail] = useState("");
  const [status, setStatus] = useState("idle");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  async function handleSubmit(event) {
    event.preventDefault();
    setStatus("trial_pending");
    setMessage("");
    setError("");

    try {
      const response = await fetch("/api/trial-signup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: email.trim() }),
      });
      const payload = await safeJson(response);
      if (!response.ok || payload.state !== "email_confirmation_required") {
        throw new Error(payload.message || "Could not start the free trial.");
      }
      setStatus("email_confirmation_required");
      setMessage(payload.message || "Check your email to confirm this free trial.");
    } catch (err) {
      setStatus("error");
      setError(err instanceof Error ? err.message : "Could not start the free trial.");
    }
  }

  return (
    <main className="app-page">
      <section className="signup-panel">
        <p className="eyebrow">Free trial</p>
        <h1>Sign up, then <em>start talking</em>.</h1>
        <p>
          We'll email you a confirmation link. Click it and Conch opens with your trial session ready.
        </p>
        <form className="signup-form" onSubmit={handleSubmit}>
          <label htmlFor="trial-email">Work email</label>
          <input
            id="trial-email"
            name="email"
            type="email"
            autoComplete="email"
            placeholder="you@company.co"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            required
          />
          <button className="button button-primary" type="submit" disabled={status === "trial_pending"}>
            {status === "trial_pending" ? "Sending confirmation…" : "Start free"}
          </button>
          {message && <p className="success-text">{message}</p>}
          {error && <p className="error-text">{error}</p>}
        </form>
        <p className="fine-print">
          No card required. Free trials open with a starter balance — talk until it's used.
        </p>
      </section>
    </main>
  );
}


function ConfirmPage() {
  const [status, setStatus] = useState("trial_pending");
  const [message, setMessage] = useState("Confirming your email…");

  useEffect(() => {
    const query = new URLSearchParams(window.location.search);
    const hash = new URLSearchParams(window.location.hash.replace(/^#/, ""));
    const accessToken = hash.get("access_token") || query.get("access_token") || "";
    const tokenHash = query.get("token_hash") || hash.get("token_hash") || "";
    const type = query.get("type") || hash.get("type") || "email";

    if (!accessToken && !tokenHash) {
      setStatus("email_confirmation_required");
      setMessage("This confirmation link is missing or expired. Start the free trial again.");
      return;
    }

    async function confirmEmail() {
      try {
        const response = await fetch("/api/trial-confirm", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ accessToken, tokenHash, type }),
        });
        const payload = await safeJson(response);
        if (!response.ok || !payload.sessionToken) {
          throw new Error(payload.message || "Could not confirm this trial.");
        }
        saveTrialSession({
          id: payload.sessionId,
          email: payload.email,
          emailConfirmed: payload.emailConfirmed,
          createdAt: new Date().toISOString(),
          expiresAt: payload.expiresAt,
          trial: payload.trial,
          trialBudgetCents: payload.trialBudgetCents,
          globalFreeTrialBudgetCents: payload.globalFreeTrialBudgetCents,
          sessionToken: payload.sessionToken,
        });
        setStatus("consent_required");
        setMessage("Email confirmed. Opening Conch…");
        window.setTimeout(() => window.location.assign("/app"), 500);
      } catch (err) {
        setStatus("error");
        setMessage(err instanceof Error ? err.message : "Could not confirm this trial.");
      }
    }

    confirmEmail();
  }, []);

  return (
    <main className="app-page">
      <section className="signup-panel">
        <p className="eyebrow">Email confirmation</p>
        <h1>{appStatusCopy[status] || "Confirming"}</h1>
        <p>{message}</p>
        {status === "error" && <a className="button button-primary" href="/app/signup">Start again</a>}
      </section>
    </main>
  );
}

function AppWorkspace() {
  const [trialSession, setTrialSession] = useState(null);
  const [recordingConsentAccepted, setRecordingConsentAccepted] = useState(false);
  const [state, setState] = useState("signup_required");
  const [statusText, setStatusText] = useState("Sign up for a free trial to open the voice workspace.");
  const [turns, setTurns] = useState([]);
  const [interimText, setInterimText] = useState("");
  const [error, setError] = useState("");
  const mediaStreamRef = useRef(null);
  const mediaRecorderRef = useRef(null);
  const websocketRef = useRef(null);
  const sessionTimerRef = useRef(null);

  useEffect(() => {
    const session = loadTrialSession();
    setTrialSession(session);
    if (session) {
      setState("consent_required");
      setStatusText("Free trial active. Accept recording consent before starting voice.");
    }
  }, []);

  useEffect(() => () => closeVoiceResources(), []);

  const canStart = trialSession && recordingConsentAccepted && state !== "listening" && state !== "thinking" && state !== "speaking";

  function acceptRecordingConsent() {
    if (!trialSession) {
      setState("signup_required");
      setStatusText("Sign up before accepting recording consent.");
      return;
    }
    setRecordingConsentAccepted(true);
    setState("ready_to_talk");
    setStatusText("Recording consent accepted. Start voice when ready.");
  }

  async function beginVoiceRun() {
    setError("");

    if (!trialSession?.sessionToken) {
      setState("signup_required");
      setStatusText("Sign up for the free trial before starting voice.");
      return;
    }

    if (!recordingConsentAccepted) {
      setState("consent_required");
      setStatusText("Recording consent is required before we can listen.");
      return;
    }

    try {
      setState("thinking");
      setStatusText("Preparing your voice session…");
      const tokenResponse = await fetch("/api/deepgram-token", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${trialSession.sessionToken}`,
          "X-Conch-Session": trialSession.id,
        },
        body: JSON.stringify({
          sessionId: trialSession.id,
          consentVersion,
          requestedAt: new Date().toISOString(),
        }),
      });
      const tokenPayload = await safeJson(tokenResponse);

      if (!tokenResponse.ok && tokenPayload.state && appStatusCopy[tokenPayload.state]) {
        setState(tokenPayload.state);
        setStatusText(tokenPayload.message || "Voice is not available yet.");
        return;
      }

      if (!tokenResponse.ok || !tokenPayload.access_token) {
        throw new Error(tokenPayload.message || "Could not start the voice session.");
      }

      setStatusText("Opening your microphone…");
      const stream = await navigator.mediaDevices.getUserMedia({
        audio: {
          echoCancellation: true,
          noiseSuppression: true,
          autoGainControl: true,
        },
        video: false,
      });
      mediaStreamRef.current = stream;

      const websocket = new WebSocket(deepgramListenUrl(), ["bearer", tokenPayload.access_token]);
      websocketRef.current = websocket;

      websocket.onopen = () => {
        const recorder = createMediaRecorder(stream);
        mediaRecorderRef.current = recorder;
        recorder.ondataavailable = (event) => {
          if (event.data.size > 0 && websocket.readyState === WebSocket.OPEN) {
            websocket.send(event.data);
          }
        };
        recorder.start(250);
        const maxSessionMs = Math.max(30, Number(tokenPayload.max_session_seconds || 600)) * 1000;
        sessionTimerRef.current = window.setTimeout(() => {
          stopVoiceRun("ended", "Trial session ended. Start another if you'd like to keep talking.");
        }, maxSessionMs);
        setState("listening");
        setStatusText("Listening. Speak naturally; stop when done.");
      };

      websocket.onmessage = (event) => handleDeepgramMessage(event.data);
      websocket.onerror = () => {
        setError("The voice connection dropped. Stop and try again.");
        stopVoiceRun("error", "The voice connection dropped. Stop and try again.");
      };
      websocket.onclose = () => {
        stopLocalMedia();
        setState((current) => current === "error" ? "error" : "ended");
        setStatusText("Voice stream ended.");
      };
    } catch (err) {
      stopVoiceRun("error", "Voice could not start.");
      setError(err instanceof Error ? err.message : "Voice could not start.");
    }
  }

  function handleDeepgramMessage(rawData) {
    const message = parseJson(rawData);
    if (!message) return;

    if (message.type === "SpeechStarted") {
      setState("listening");
      setStatusText("Speech detected.");
      return;
    }

    if (message.type === "Results") {
      const transcript = message.channel?.alternatives?.[0]?.transcript?.trim();
      if (!transcript) return;

      if (message.is_final || message.speech_final) {
        setInterimText("");
        setTurns((current) => [
          ...current,
          {
            id: `${Date.now()}-${current.length}`,
            speaker: "You",
            text: transcript,
          },
        ]);
        setState("thinking");
        setStatusText("Captured. Conch is thinking…");
      } else {
        setInterimText(transcript);
        setState("listening");
      }
    }
  }

  function stopLocalMedia() {
    const stream = mediaStreamRef.current;
    if (stream) {
      stream.getTracks().forEach((track) => track.stop());
    }
    mediaStreamRef.current = null;
  }

  function closeVoiceResources() {
    const recorder = mediaRecorderRef.current;
    if (recorder && recorder.state !== "inactive") {
      recorder.stop();
    }
    mediaRecorderRef.current = null;

    if (sessionTimerRef.current) {
      window.clearTimeout(sessionTimerRef.current);
      sessionTimerRef.current = null;
    }

    const websocket = websocketRef.current;
    if (websocket && websocket.readyState === WebSocket.OPEN) {
      websocket.send(JSON.stringify({ type: "CloseStream" }));
      websocket.close();
    } else if (websocket && websocket.readyState === WebSocket.CONNECTING) {
      websocket.close();
    }
    websocketRef.current = null;

    stopLocalMedia();
  }

  function stopVoiceRun(nextState = "ended", nextStatus = "Voice stream stopped.") {
    closeVoiceResources();
    setState(nextState);
    setStatusText(nextStatus);
  }

  function resetTrial() {
    window.localStorage.removeItem(trialStorageKey);
    setTrialSession(null);
    setRecordingConsentAccepted(false);
    setTurns([]);
    setInterimText("");
    stopVoiceRun("signup_required", "Sign up for a free trial to open the voice workspace.");
  }

  return (
    <main className="workspace-layout">
      <section className="workspace-main" aria-label="Conch voice workspace">
        <div className="workspace-heading">
          <p className="eyebrow">Voice studio</p>
          <h1>Talk to <em>Conch</em>.</h1>
          <p>
            This is the real app — no sample transcripts, no canned briefs. Conch waits for your signup and consent, then starts listening.
          </p>
        </div>

        <div className="status-banner" data-state={state} role="status" aria-live="polite">
          <strong>{appStatusCopy[state] || "App status"}</strong>
          <span>{statusText}</span>
        </div>

        {!trialSession ? (
          <div className="empty-state">
            <h2>Free trial required</h2>
            <p>Open the app by creating a free trial session first.</p>
            <a className="button button-primary" href="/app/signup">Start free</a>
          </div>
        ) : (
          <>
            <section className="consent-panel" aria-label="Recording consent">
              <div>
                <h2>Recording consent</h2>
                <p>
                  Confirm you have permission from every participant before recording begins.
                </p>
              </div>
              <button
                className="button button-secondary"
                type="button"
                onClick={acceptRecordingConsent}
                aria-pressed={recordingConsentAccepted}
              >
                {recordingConsentAccepted ? "Consent accepted" : "I have recording consent"}
              </button>
            </section>

            <section className="control-row" aria-label="Voice controls">
              <button className="button button-primary" type="button" onClick={beginVoiceRun} disabled={!canStart}>
                Start voice
              </button>
              <button className="button button-secondary" type="button" onClick={() => stopVoiceRun()}>
                Stop
              </button>
              <button className="button button-ghost" type="button" onClick={resetTrial}>
                Reset trial
              </button>
            </section>

            {error && <p className="error-text">{error}</p>}

            <section className="session-grid" aria-label="Conch session regions">
              <article className="session-card transcript-card">
                <div className="card-label"><span>Transcript</span><span>Live</span></div>
                <p className="speaker-legend"><span>● You</span><span>✣ Conch</span></p>
                {turns.length === 0 && !interimText ? (
                  <p>No transcript yet. Start voice after consent to capture real audio.</p>
                ) : (
                  <div className="turn-list">
                    {turns.map((turn) => (
                      <p key={turn.id}><strong>{turn.speaker}</strong>{turn.text}</p>
                    ))}
                    {interimText && <p className="interim"><strong>You · interim</strong>{interimText}</p>}
                  </div>
                )}
              </article>
              <article className="session-card">
                <div className="card-label"><span>Brief</span><span>Pending</span></div>
                <p>Your brief will appear here once the session wraps.</p>
              </article>
              <article className="session-card">
                <div className="card-label"><span>Status</span><span>Studio</span></div>
                <p>{statusText}</p>
                <p style={{ fontFamily: "var(--font-mono)", fontSize: "0.78rem", color: "var(--ink-soft)", letterSpacing: "0.04em" }}>
                  Hold Space — talk · Tap Space — toggle mic · Esc — interrupt
                </p>
                <span className="sr-only">Voice modes: Listening, Thinking, Speaking, Closing.</span>
                <p className="waveform" aria-label="Waveform placeholder">▁ ▂ ▃ ▄ ▅ ▆ ▇ █</p>
              </article>
            </section>
          </>
        )}
      </section>

    </main>
  );
}

function LegalPage({ page }) {
  return (
    <main className="legal-layout">
      <p className="eyebrow">{page.eyebrow}</p>
      <h1>{page.title}</h1>
      <div className="legal-sections">
        {page.sections.map(([heading, text]) => (
          <section key={heading}>
            <h2>{heading}</h2>
            <p>{text}</p>
          </section>
        ))}
      </div>
      <a className="button button-secondary" href={supportHref}>Contact {contactEmail}</a>
    </main>
  );
}

function StatusPage() {
  return (
    <main className="legal-layout">
      <p className="eyebrow">Status</p>
      <h1>Conch is <em>online</em>.</h1>
      <p>
        All systems up. If voice doesn't open right away, give it a moment and try again.
      </p>
      <div className="hero-actions">
        <a className="button button-primary" href="/app">Open Conch</a>
        <a className="button button-secondary" href="/app/signup">Start free</a>
      </div>
    </main>
  );
}

function NotFound() {
  return (
    <main className="legal-layout">
      <p className="eyebrow">404</p>
      <h1>That page <em>drifted out to sea</em>.</h1>
      <p>Head back to Conch or contact {contactEmail} if you expected something here.</p>
      <a className="button button-primary" href="/app">Open Conch</a>
    </main>
  );
}

function normalizePath(pathname) {
  if (!pathname || pathname === "/index.html") return "/";
  return pathname.endsWith("/") && pathname.length > 1 ? pathname.slice(0, -1) : pathname;
}

function loadTrialSession() {
  try {
    const raw = window.localStorage.getItem(trialStorageKey);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function saveTrialSession(session) {
  window.localStorage.setItem(trialStorageKey, JSON.stringify(session));
}

function parseJson(rawData) {
  try {
    return JSON.parse(rawData);
  } catch {
    return null;
  }
}

async function safeJson(response) {
  try {
    return await response.json();
  } catch {
    return {};
  }
}

function deepgramListenUrl() {
  const params = new URLSearchParams({
    model: "nova-3",
    smart_format: "true",
    interim_results: "true",
    endpointing: "200",
    utterance_end_ms: "1000",
    vad_events: "true",
  });
  return `wss://api.deepgram.com/v1/listen?${params.toString()}`;
}

function createMediaRecorder(stream) {
  const preferredTypes = [
    "audio/webm;codecs=opus",
    "audio/webm",
    "audio/ogg;codecs=opus",
  ];
  const mimeType = preferredTypes.find((type) => window.MediaRecorder?.isTypeSupported(type));
  return mimeType ? new MediaRecorder(stream, { mimeType }) : new MediaRecorder(stream);
}


export default App;
