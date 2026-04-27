import { useEffect, useState } from "react";

const contactEmail = "conch@theos.sh";
const billingHref = `mailto:${contactEmail}?subject=Conch%20billing`;
const supportHref = `mailto:${contactEmail}?subject=Conch%20support`;
const cargoInstall = "cargo install --git https://github.com/nyx-haile/conch";

const productHighlights = [
  {
    no: "01",
    title: "Type a topic, talk it through",
    text: "Run conch talk \"your topic\" and Conch drives a spoken Q&A, then writes the brief when you're done.",
  },
  {
    no: "02",
    title: "Three depths",
    text: "Sketch for fast, talk for default, chronicle for deep. Pick the one that fits the problem.",
  },
  {
    no: "03",
    title: "Local-first",
    text: "Local mic, live transcript on screen, written brief saved to your machine on exit.",
  },
];

const usageOptions = [
  {
    no: "01",
    title: "Free trial",
    text: "Confirm your email and Conch issues an API key with a starter balance. No card, no auto-renew.",
  },
  {
    no: "02",
    title: "Bring your own keys",
    text: "Prefer to route through your own provider account? Set the env vars and Conch uses your accounts directly.",
  },
  {
    no: "03",
    title: "Pay as you talk",
    text: "Past the trial, you only pay for actual session time.",
  },
];

const downloadTargets = [
  { no: "01", platform: "macOS · Apple silicon", arch: "aarch64-apple-darwin" },
  { no: "02", platform: "macOS · Intel", arch: "x86_64-apple-darwin" },
  { no: "03", platform: "Linux · x86_64", arch: "x86_64-unknown-linux-gnu" },
  { no: "04", platform: "Linux · aarch64", arch: "aarch64-unknown-linux-gnu" },
  { no: "05", platform: "Windows · x86_64", arch: "x86_64-pc-windows-msvc" },
];

const legalPages = {
  "/terms.html": {
    title: "Terms of Service",
    eyebrow: "Version 2026-04-27",
    sections: [
      [
        "Beta status",
        "Conch is an early-beta voice-interview CLI. Email confirmation is required to receive an API key when the managed tier opens; long-term storage and exports are coming soon.",
      ],
      [
        "Payments",
        "Pricing is bring-your-own-keys or pay-as-you-go for Conch-managed voice and model usage. Confirmed free trials include $10 of starter usage per user and pause when aggregate free-trial usage reaches $10,000. No subscription. No automatic charge. Conch charges only after a separate explicit purchase or usage agreement.",
      ],
      [
        "Recording responsibilities",
        "You are responsible for obtaining every consent required by law before recording or transcribing anyone with the Conch CLI. Do not use Conch for unlawful surveillance, biometric identification, children under 13, or regulated workflows without written approval.",
      ],
      [
        "Data rights",
        `You may request export, deletion, saved-card removal, or billing help at ${contactEmail}.`,
      ],
    ],
  },
  "/privacy.html": {
    title: "Privacy Policy",
    eyebrow: "Version 2026-04-27",
    sections: [
      [
        "What Conch needs",
        "Conch collects the minimum account, email, session, consent, audio, transcript, brief, and log data needed to run the service.",
      ],
      [
        "Processors",
        "Deepgram may process audio for speech-to-text and voice features when you use the Conch CLI in managed mode. Payment processors may process billing metadata only when a paid workflow is in use.",
      ],
      [
        "This site",
        "This site is a download portal and account page. It does not request microphone access. The Conch CLI requests microphone access on your own machine only when you start a voice session there.",
      ],
      [
        "Contact",
        `Contact ${contactEmail} for export, deletion, billing, or saved-card removal requests.`,
      ],
    ],
  },
  "/recording-consent.html": {
    title: "Recording Consent Notice",
    eyebrow: "Version 2026-04-27",
    sections: [
      [
        "Before recording",
        "The Conch CLI records and transcribes audio only when you start a voice session on your own machine. Recording laws vary by location. Some places require consent from every participant.",
      ],
      [
        "Your confirmation",
        "By starting a voice session, you confirm that you have the right to record and transcribe the conversation and that every required participant has consented.",
      ],
      [
        "In-CLI consent prompt",
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

function TerminalFrame() {
  return (
    <aside className="terminal-frame" aria-label="Conch terminal preview">
      <div className="terminal-bar">
        <span className="terminal-dot" data-tone="rest" />
        <span className="terminal-dot" data-tone="warn" />
        <span className="terminal-dot" data-tone="go" />
        <span className="terminal-title">conch</span>
      </div>
      <pre className="terminal-screen">
        <span className="terminal-line">
          <span className="terminal-prompt">~ ❯</span> conch talk{" "}
          <span className="terminal-arg">"voice discovery, briefly"</span>
        </span>
        <span className="terminal-line terminal-meta">
          <span className="terminal-mark">✣</span> Conch · listening
        </span>
        <span className="terminal-line terminal-wave">▁ ▂ ▃ ▄ ▅ ▆ ▇ █ ▇ ▆ ▅ ▄ ▃ ▂</span>
        <span className="terminal-line terminal-dialogue">
          &gt;{" "}
          <span className="terminal-italic">
            Who's the brief for, and what should they walk away knowing?
          </span>
        </span>
        <span className="terminal-line terminal-cursor">
          <span className="terminal-prompt">~ ❯</span>{" "}
          <span className="terminal-blink">▌</span>
        </span>
      </pre>
    </aside>
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

  if (path === "/download" || path === "/cli") {
    return <Shell active="download"><DownloadPage /></Shell>;
  }

  if (path === "/account") {
    return <Shell active="account"><AccountPage /></Shell>;
  }

  if (path === "/account/confirm") {
    return <Shell active="account"><AccountConfirmPage /></Shell>;
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
          <a className={active === "download" ? "active" : ""} href="/download">Download</a>
          <a className={active === "account" ? "active" : ""} href="/account">Account</a>
          <a href="/#usage">Usage</a>
          <a href="/recording-consent.html">Consent</a>
          <a href="/privacy.html">Privacy</a>
        </nav>
      </header>
      {children}
      <footer className="site-footer">
        <span>© 2026 Conch — voice discovery, briefly.</span>
        <a href="/download">Download</a>
        <a href="/account">Account</a>
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
          <h1>Type a topic and <em>start talking.</em></h1>
          <p className="hero-lede">
            Conch is a voice-interview CLI. Install the binary, paste your API key, run{" "}
            <code>conch talk</code> and Conch turns a spoken conversation into a launch-ready brief.
          </p>
          <div className="hero-actions">
            <a className="button button-primary" href="/download">Download</a>
            <a className="button button-secondary" href="/account">Get API key</a>
          </div>
          <p className="trust-note">
            Bring your own keys, or pay as you talk. No subscription. No automatic charge.
          </p>
        </div>

        <aside className="app-preview" aria-label="Conch install path">
          <div className="preview-illustration">
            <ConchSpiral />
          </div>
          <div className="preview-topline">
            <span>Install path</span>
            <span>CLI-ready</span>
          </div>
          <ol className="path-list">
            <li><strong>i</strong><span>Download Conch for your platform.</span></li>
            <li><strong>ii</strong><span>Confirm your email to claim an API key.</span></li>
            <li><strong>iii</strong><span>Run <code>conch talk "topic"</code> and start talking.</span></li>
          </ol>
          <a className="button button-primary full-width" href="/download">Get Conch</a>
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
            The first product job is access: download the binary, claim a free trial, and talk. Free trials open with a starter balance — past that, you pay only for what you use.
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
          <h2>Conch lives in your <em>terminal</em>.</h2>
        </div>
        <div className="hero-actions">
          <a className="button button-primary" href="/download">Download</a>
          <a className="button button-secondary" href="/account">Get API key</a>
        </div>
      </section>
    </main>
  );
}

function DownloadPage() {
  return (
    <main className="cli-page">
      <section className="cli-hero section-frame">
        <div className="cli-copy">
          <p className="eyebrow">Download · Conch CLI</p>
          <h1>Conch in your <em>terminal</em>.</h1>
          <p className="cli-lede">
            One binary. Type a topic, talk it through, walk away with a written brief. Pick your platform below — signed builds land with the next release.
          </p>
          <div className="hero-actions">
            <a className="button button-primary" href="/account">Get API key</a>
            <a className="button button-secondary" href="#install">Install from source</a>
          </div>
        </div>
        <TerminalFrame />
      </section>

      <section className="download-section">
        <div className="download-heading">
          <p className="eyebrow">Binaries</p>
          <h2>Pick your platform.</h2>
          <p>
            Builds for the major desktops are landing in the next release. Drop your email on the Account page and we'll ping you the moment they're up.
          </p>
        </div>
        <ol className="download-table" aria-label="Conch platform binaries">
          {downloadTargets.map((target) => (
            <li key={target.arch} className="download-row">
              <span className="download-no">{target.no}</span>
              <span className="download-platform">{target.platform}</span>
              <span className="download-arch">{target.arch}</span>
              <span className="download-state">Soon</span>
            </li>
          ))}
        </ol>
      </section>

      <section className="install-section" id="install">
        <div className="install-heading">
          <p className="eyebrow">Install from source</p>
          <h2>Or build it <em>yourself</em>.</h2>
          <p>
            Rust users can install Conch straight from the repo with one Cargo command.
          </p>
        </div>
        <div className="install-blocks">
          <div className="install-block">
            <p className="install-label">Cargo</p>
            <pre><code>{cargoInstall}</code></pre>
          </div>
        </div>
      </section>

      <section className="cli-callout">
        <div>
          <p className="eyebrow">Then</p>
          <h2>Paste your <em>key</em> and talk.</h2>
          <p>Set <code>CONCH_API_KEY</code> or run <code>conch login</code>, and you're in.</p>
        </div>
        <div className="hero-actions">
          <a className="button button-primary" href="/account">Get API key</a>
        </div>
      </section>
    </main>
  );
}

function AccountPage() {
  const [email, setEmail] = useState("");
  const [status, setStatus] = useState("idle");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  async function handleSubmit(event) {
    event.preventDefault();
    setStatus("pending");
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
        throw new Error(payload.message || "Could not start your account.");
      }
      setStatus("sent");
      setMessage(payload.message || "Check your email to confirm your account.");
    } catch (err) {
      setStatus("error");
      setError(err instanceof Error ? err.message : "Could not start your account.");
    }
  }

  return (
    <main className="app-page">
      <section className="signup-panel">
        <p className="eyebrow">Account</p>
        <h1>Get your <em>API key</em>.</h1>
        <p>
          Confirm your email and Conch will issue a key when the managed tier opens. No card. No auto-renew.
        </p>
        <form className="signup-form" onSubmit={handleSubmit}>
          <label htmlFor="account-email">Email</label>
          <input
            id="account-email"
            name="email"
            type="email"
            autoComplete="email"
            placeholder="you@company.co"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            required
          />
          <button className="button button-primary" type="submit" disabled={status === "pending"}>
            {status === "pending" ? "Sending confirmation…" : "Email me a link"}
          </button>
          {message && <p className="success-text">{message}</p>}
          {error && <p className="error-text">{error}</p>}
        </form>
        <p className="fine-print">
          We email a one-time confirmation link. After you click it, your API key (or a notice when keys ship) lands at this address.
        </p>
      </section>
    </main>
  );
}

function AccountConfirmPage() {
  const [status, setStatus] = useState("pending");
  const [message, setMessage] = useState("Confirming your email…");

  useEffect(() => {
    const query = new URLSearchParams(window.location.search);
    const hash = new URLSearchParams(window.location.hash.replace(/^#/, ""));
    const accessToken = hash.get("access_token") || query.get("access_token") || "";
    const tokenHash = query.get("token_hash") || hash.get("token_hash") || "";
    const type = query.get("type") || hash.get("type") || "email";

    if (!accessToken && !tokenHash) {
      setStatus("error");
      setMessage("This confirmation link is missing or expired. Start the account flow again.");
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
        if (!response.ok || !payload.email) {
          throw new Error(payload.message || "Could not confirm your account.");
        }
        setStatus("confirmed");
        setMessage(`You're confirmed as ${payload.email}. We'll email your API key the moment the managed tier opens.`);
      } catch (err) {
        setStatus("error");
        setMessage(err instanceof Error ? err.message : "Could not confirm your account.");
      }
    }

    confirmEmail();
  }, []);

  const heading = status === "confirmed"
    ? "You're in."
    : status === "error"
      ? "We hit a snag."
      : "Confirming…";

  return (
    <main className="app-page">
      <section className="signup-panel">
        <p className="eyebrow">Account confirmation</p>
        <h1>{heading}</h1>
        <p>{message}</p>
        {status === "error" && (
          <a className="button button-primary" href="/account">Start again</a>
        )}
        {status === "confirmed" && (
          <a className="button button-primary" href="/download">Download Conch</a>
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
        Site is up. Binary builds and managed-tier API keys are landing with the next release — drop your email on the Account page to get pinged.
      </p>
      <div className="hero-actions">
        <a className="button button-primary" href="/download">Download</a>
        <a className="button button-secondary" href="/account">Get API key</a>
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
      <a className="button button-primary" href="/download">Download Conch</a>
    </main>
  );
}

function normalizePath(pathname) {
  if (!pathname || pathname === "/index.html") return "/";
  return pathname.endsWith("/") && pathname.length > 1 ? pathname.slice(0, -1) : pathname;
}

async function safeJson(response) {
  try {
    return await response.json();
  } catch {
    return {};
  }
}

export default App;
