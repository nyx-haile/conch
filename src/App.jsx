import { useEffect, useRef, useState } from "react";

const contactEmail = "conch@theos.sh";
const billingHref = `mailto:${contactEmail}?subject=Conch%20billing`;
const supportHref = `mailto:${contactEmail}?subject=Conch%20support`;
const trialStorageKey = "conch_trial_session_v1";
const consentVersion = "2026-04-26";

const appStates = [
  "signup_required",
  "trial_pending",
  "voice_config_missing",
  "consent_required",
  "ready_to_talk",
  "listening",
  "thinking",
  "speaking",
  "ended",
  "error",
];

const productHighlights = [
  {
    title: "Open the app first",
    text: "Start at /app, sign up for the free trial, accept recording consent, and begin a real voice session when Deepgram is configured.",
  },
  {
    title: "Deepgram underneath",
    text: "Browser audio uses a short-lived token from Conch. Server credentials stay off the client bundle.",
  },
  {
    title: "No billing theater",
    text: "Conch supports BYOK where possible and usage-based managed voice/model billing. No subscription. No automatic charge.",
  },
];

const usageOptions = [
  {
    title: "Free trial",
    text: "Create a trial session and land directly in the app. If voice is configured, the next step is consent and talking.",
  },
  {
    title: "BYOK",
    text: "Bring provider/model keys where Conch supports it. Keys remain server-side or in your controlled runtime.",
  },
  {
    title: "Usage-based",
    text: "Conch-managed Deepgram and model usage can be metered by actual session activity instead of fixed public tiers.",
  },
];

const legalPages = {
  "/terms.html": {
    title: "Terms of Service",
    eyebrow: "Version 2026-04-26",
    sections: [
      [
        "Beta status",
        "Conch is an early beta voice-discovery app. The web app can start a trial session, while durable billing, exports, and long-term storage remain backend milestones.",
      ],
      [
        "Payments",
        "Pricing is BYOK and/or usage-based for Conch-managed voice and model usage. No subscription. No automatic charge. Conch charges only after a separate explicit purchase or managed-usage agreement.",
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
        "Conch collects the minimum account, session, consent, audio, transcript, generated brief, provider-routing metadata, and logs needed to run the service.",
      ],
      [
        "Processors",
        "Deepgram may process audio for cloud speech-to-text and voice features when cloud mode is enabled. Payment processors may process billing metadata only when a separate paid workflow is enabled.",
      ],
      [
        "This public app",
        "This app requests microphone access only after you accept the recording-consent gate and only after the deployment can issue a short-lived Deepgram token.",
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
        "Deepgram receives audio only after consent and only through Conch's short-lived access path or server proxy.",
      ],
    ],
  },
};

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
          <span className="brand-mark">◒</span>
          <span>Conch</span>
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
        <span>© 2026 Conch</span>
        <a href="/terms.html">Terms</a>
        <a href="/privacy.html">Privacy</a>
        <a href="/recording-consent.html">Recording Consent</a>
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
          <p className="eyebrow">Voice discovery for builders</p>
          <h1>Open Conch and start talking.</h1>
          <p className="hero-lede">
            Conch turns a live voice conversation into a product brief. Start a free trial,
            accept recording consent, and use Deepgram-backed voice when this deployment is configured.
          </p>
          <div className="hero-actions">
            <a className="button button-primary" href="/app/signup">Start free</a>
            <a className="button button-secondary" href="/app">Open Conch</a>
          </div>
          <p className="trust-note">
            BYOK where supported, or usage-based billing for Conch-managed voice and model usage. No subscription. No automatic charge.
          </p>
        </div>

        <aside className="app-preview" aria-label="Conch app access path">
          <div className="preview-topline">
            <span>App path</span>
            <span>Deepgram-ready</span>
          </div>
          <ol className="path-list">
            <li><strong>1</strong><span>Sign up for the free trial.</span></li>
            <li><strong>2</strong><span>Accept recording consent.</span></li>
            <li><strong>3</strong><span>Start a real voice session — no fake transcript.</span></li>
          </ol>
          <a className="button button-primary full-width" href="/app">Go to app</a>
        </aside>
      </section>

      <section className="bento-grid" aria-label="Conch product highlights">
        {productHighlights.map((feature) => (
          <article className="bento-card" key={feature.title}>
            <h2>{feature.title}</h2>
            <p>{feature.text}</p>
          </article>
        ))}
      </section>

      <section className="section-frame usage-section" id="usage">
        <div>
          <p className="eyebrow">BYOK / Usage-based</p>
          <h2>No public price theater.</h2>
          <p>
            The first product job is access: open the app, start a free trial, and talk. Billing can be BYOK or metered by actual managed usage.
          </p>
        </div>
        <div className="usage-grid">
          {usageOptions.map((option) => (
            <article className="usage-card" key={option.title}>
              <h3>{option.title}</h3>
              <p>{option.text}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="cta-strip">
        <div>
          <p className="eyebrow">Ready now</p>
          <h2>The app is the front door.</h2>
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
  const [error, setError] = useState("");

  async function handleSubmit(event) {
    event.preventDefault();
    setStatus("trial_pending");
    setError("");

    try {
      const response = await fetch("/api/trial-signup", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: email.trim() }),
      });
      const payload = await safeJson(response);
      if (!response.ok || !payload.sessionToken) {
        throw new Error(payload.message || "Could not start the free trial.");
      }
      saveTrialSession({
        id: payload.sessionId,
        email: payload.email,
        createdAt: new Date().toISOString(),
        expiresAt: payload.expiresAt,
        trial: payload.trial,
        sessionToken: payload.sessionToken,
      });
      window.location.assign("/app");
    } catch (err) {
      setStatus("error");
      setError(err instanceof Error ? err.message : "Could not start the free trial.");
    }
  }

  return (
    <main className="app-page section-frame">
      <section className="signup-panel">
        <p className="eyebrow">Free trial</p>
        <h1>Sign up, then start talking.</h1>
        <p>
          This requests a server-issued trial session and sends you into `/app`. When Deepgram is configured on the deployment, Conch can request a short-lived voice token after consent.
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
            {status === "trial_pending" ? "Opening app…" : "Start free"}
          </button>
          {error && <p className="error-text">{error}</p>}
        </form>
        <p className="fine-print">
          No card collection in this app shell. BYOK or usage-based billing is handled separately through {contactEmail}.
        </p>
      </section>
      <aside className="state-card">
        <h2>Trial flow</h2>
        <StateList active={status === "trial_pending" ? "trial_pending" : "signup_required"} />
      </aside>
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
      setStatusText("Recording consent is required before microphone capture or Deepgram streaming.");
      return;
    }

    try {
      setState("thinking");
      setStatusText("Requesting a short-lived Deepgram voice token from Conch…");
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

      if (tokenResponse.status === 503 || tokenPayload.state === "voice_config_missing") {
        setState("voice_config_missing");
        setStatusText(tokenPayload.message || "Deepgram voice is not configured on this deployment yet.");
        return;
      }

      if (!tokenResponse.ok || !tokenPayload.access_token) {
        throw new Error(tokenPayload.message || "Could not start Deepgram voice.");
      }

      setStatusText("Token ready. Opening microphone after consent…");
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
        setState("listening");
        setStatusText("Listening with Deepgram. Speak naturally; stop when done.");
      };

      websocket.onmessage = (event) => handleDeepgramMessage(event.data);
      websocket.onerror = () => {
        setError("Deepgram WebSocket failed. Stop the run and try again.");
        stopVoiceRun("error", "Deepgram WebSocket failed. Stop the run and try again.");
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
            speaker: "● You",
            text: transcript,
          },
        ]);
        setState("thinking");
        setStatusText("Captured a final transcript turn. Conch backend response is pending durable session wiring.");
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
      <section className="workspace-main section-frame" aria-label="Conch voice workspace">
        <div className="workspace-heading">
          <p className="eyebrow">/app</p>
          <h1>Talk to Conch.</h1>
          <p>
            This is the real app entry. It does not show sample transcripts or fake briefs: it waits for signup, recording consent, and Deepgram configuration.
          </p>
        </div>

        <div className="status-banner" data-state={state} role="status" aria-live="polite">
          <strong>{state}</strong>
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
                  Confirm you have permission from every participant before microphone capture or Deepgram streaming begins.
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
                <div className="card-label">Transcript</div>
                {turns.length === 0 && !interimText ? (
                  <p>No transcript yet. Start voice after consent to capture real audio.</p>
                ) : (
                  <div className="turn-list">
                    {turns.map((turn) => (
                      <p key={turn.id}><strong>{turn.speaker}</strong> {turn.text}</p>
                    ))}
                    {interimText && <p className="interim"><strong>● You</strong> {interimText}</p>}
                  </div>
                )}
              </article>
              <article className="session-card">
                <div className="card-label">Brief</div>
                <p>Generated brief output will appear only after durable Conch session wiring is complete.</p>
              </article>
              <article className="session-card">
                <div className="card-label">Status</div>
                <p>✣ Conch: {statusText}</p>
                <p>Controls: Hold Space talk, Tap Space toggle mic, Esc interrupt. States: Listening, Thinking, Speaking, Closing.</p>
                <p className="waveform" aria-label="Waveform placeholder">▁ ▂ ▃ ▄ ▅ ▆ ▇ █</p>
              </article>
            </section>
          </>
        )}
      </section>

      <aside className="state-card section-frame">
        <h2>Finite app states</h2>
        <StateList active={state} />
        <p className="fine-print">
          Session: {trialSession ? trialSession.id : "none"}
        </p>
      </aside>
    </main>
  );
}

function StateList({ active }) {
  return (
    <ol className="state-list">
      {appStates.map((item) => (
        <li className={item === active ? "current" : ""} key={item}>{item}</li>
      ))}
    </ol>
  );
}

function LegalPage({ page }) {
  return (
    <main className="legal-layout section-frame">
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
    <main className="legal-layout section-frame">
      <p className="eyebrow">Public app status</p>
      <h1>Conch is online.</h1>
      <p>
        The app-first web surface is serving `/`, `/app`, `/app/signup`, legal, consent, and status routes. Voice availability depends on the server-side Deepgram token broker configuration.
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
    <main className="legal-layout section-frame">
      <p className="eyebrow">404</p>
      <h1>That page drifted out to sea.</h1>
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
