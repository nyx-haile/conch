import { motion } from "motion/react";
import { BackgroundBeams } from "./components/BackgroundBeams.jsx";

const contactEmail = "conch@theos.sh";
const contactHref = `mailto:${contactEmail}?subject=Conch%20beta%20access`;

const plans = [
  {
    name: "Free trial",
    price: "$0",
    detail: "120 cloud STT minutes for 14 days",
    note: "Card required once checkout is enabled. No automatic charge.",
  },
  {
    name: "Starter",
    price: "$29",
    detail: "1,000 cloud STT minutes",
    note: "One-time prepaid pack for early product discovery.",
  },
  {
    name: "Team",
    price: "$199",
    detail: "10,000 cloud STT minutes",
    note: "Shared credits for a small product team.",
    featured: true,
  },
  {
    name: "Pilot",
    price: "$499",
    detail: "30,000 cloud STT minutes plus onboarding",
    note: "For teams bringing Conch into a launch process.",
  },
];

const features = [
  {
    eyebrow: "01",
    title: "Guided discovery",
    text: "Conch keeps the conversation moving, asks sharper follow-ups, and turns rough thinking into usable requirements.",
  },
  {
    eyebrow: "02",
    title: "Launch-ready briefs",
    text: "Capture goals, constraints, risks, and open questions in a structured artifact your team can act on immediately.",
  },
  {
    eyebrow: "03",
    title: "Consent-first audio",
    text: "Cloud transcription is treated as a deliberate action with clear recording consent and server-only provider keys.",
  },
];

const legalPages = {
  "/terms.html": {
    title: "Terms of Service",
    eyebrow: "Version 2026-04-26",
    sections: [
      [
        "Beta status",
        "Conch is an early beta voice-discovery service. This deployment is a public information and contact surface, not an active payment checkout.",
      ],
      [
        "Payments",
        "Card required once checkout is enabled. No subscription. No automatic charge. A saved payment method from the free trial is only a card-verification signal. Conch charges you only when you separately buy prepaid usage credits.",
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
        "Conch collects the minimum account, Stripe customer/payment metadata, audio, transcript, generated brief, provider-routing metadata, and logs needed to run the service.",
      ],
      [
        "Processors",
        "Deepgram may process audio for cloud speech-to-text when cloud mode is enabled. Stripe processes payments and stores card details; raw card data does not touch Conch servers.",
      ],
      [
        "This public app",
        "This deployment does not collect microphone audio, payment card data, or provider API keys. Provider API keys are server-only and are not exposed to browser code.",
      ],
      [
        "Contact",
        `Contact ${contactEmail} for export, deletion, or saved-card removal requests.`,
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
        "Current launch surface",
        "This public app does not request microphone access. The product must show a consent gate before any microphone capture or Deepgram streaming.",
      ],
      [
        "In-app consent label",
        "I have consent — start recording.",
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

  if (path !== "/") {
    return <Shell><NotFound /></Shell>;
  }

  return <Shell><HomePage /></Shell>;
}

function Shell({ children }) {
  return (
    <div className="app-shell">
      <BackgroundBeams />
      <header className="site-header">
        <a href="/" className="brand" aria-label="Conch home">
          <span className="brand-mark">◒</span>
          <span>Conch</span>
        </a>
        <nav aria-label="Primary navigation">
          <a href="/#pricing">Pricing</a>
          <a href="/privacy.html">Privacy</a>
          <a href="/recording-consent.html">Consent</a>
          <a className="nav-pill" href={contactHref}>Request beta</a>
        </nav>
      </header>
      {children}
      <footer className="site-footer">
        <span>© 2026 Conch</span>
        <a href="/terms.html">Terms</a>
        <a href="/privacy.html">Privacy</a>
        <a href="/recording-consent.html">Recording Consent</a>
        <a href="/status.html">Status</a>
        <a href={`mailto:${contactEmail}?subject=Conch%20billing`}>Billing</a>
      </footer>
    </div>
  );
}

function HomePage() {
  return (
    <main>
      <section className="hero-section section-frame">
        <motion.div
          className="hero-copy"
          initial={{ opacity: 0, y: 18 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.75, ease: "easeOut" }}
        >
          <p className="eyebrow">Voice discovery for builders</p>
          <h1>Turn spoken product thinking into launch-ready briefs.</h1>
          <p className="hero-lede">
            Conch guides a live voice conversation, keeps the messy parts, and leaves you with the goals,
            constraints, risks, and decisions your team needs next.
          </p>
          <div className="hero-actions">
            <a className="button button-primary" href={contactHref}>Request beta access</a>
            <a className="button button-secondary" href="#pricing">View prepaid plans</a>
          </div>
          <p className="trust-note">
            Card required once checkout is enabled. No subscription. No automatic charge.
          </p>
        </motion.div>

        <motion.aside
          className="brief-card moving-border"
          initial={{ opacity: 0, y: 24, rotateX: 8 }}
          animate={{ opacity: 1, y: 0, rotateX: 0 }}
          transition={{ duration: 0.9, delay: 0.1, ease: "easeOut" }}
          aria-label="Example Conch brief"
        >
          <div className="card-topline">
            <span>Live brief</span>
            <span className="pulse-dot" />
          </div>
          <h2>Launch brief / Checkout beta</h2>
          <dl>
            <div>
              <dt>Goal</dt>
              <dd>Validate prepaid voice credits without recurring billing.</dd>
            </div>
            <div>
              <dt>Risk</dt>
              <dd>Recording consent must be explicit before audio capture.</dd>
            </div>
            <div>
              <dt>Next</dt>
              <dd>Invite three beta teams and review discovery quality.</dd>
            </div>
          </dl>
        </motion.aside>
      </section>

      <section className="bento-grid" aria-label="Conch product highlights">
        {features.map((feature, index) => (
          <motion.article
            className="bento-card"
            key={feature.title}
            initial={{ opacity: 0, y: 24 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-80px" }}
            transition={{ duration: 0.55, delay: index * 0.08 }}
          >
            <span>{feature.eyebrow}</span>
            <h2>{feature.title}</h2>
            <p>{feature.text}</p>
          </motion.article>
        ))}
      </section>

      <section className="section-frame pricing-section" id="pricing">
        <div>
          <p className="eyebrow">Prepaid usage</p>
          <h2>Simple packs before subscriptions ever enter the room.</h2>
          <p>
            Usage credits only count cloud speech-to-text minutes processed through Conch. Local/offline runs do not consume paid credits.
          </p>
        </div>
        <div className="pricing-grid">
          {plans.map((plan) => (
            <article className={plan.featured ? "price-card featured" : "price-card"} key={plan.name}>
              <span>{plan.name}</span>
              <strong>{plan.price}</strong>
              <p>{plan.detail}</p>
              <small>{plan.note}</small>
              <a href={`mailto:${contactEmail}?subject=Conch%20${encodeURIComponent(plan.name)}%20pack`}>
                Contact to buy
              </a>
            </article>
          ))}
        </div>
      </section>

      <section className="cta-strip">
        <div>
          <p className="eyebrow">Ready when the team is</p>
          <h2>Bring a thorny product idea. Leave with the brief.</h2>
        </div>
        <a className="button button-primary" href={contactHref}>Get beta access</a>
      </section>
    </main>
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
      <a className="button button-secondary" href={`mailto:${contactEmail}?subject=Conch%20support`}>Contact {contactEmail}</a>
    </main>
  );
}

function StatusPage() {
  return (
    <main className="legal-layout section-frame">
      <p className="eyebrow">Public beta status</p>
      <h1>Conch is online.</h1>
      <p>
        This JavaScript launch app is serving the public beta surface. For beta access, billing,
        privacy, deletion, export, or incident questions, contact {contactEmail}.
      </p>
      <a className="button button-primary" href={contactHref}>Request beta access</a>
    </main>
  );
}

function NotFound() {
  return (
    <main className="legal-layout section-frame">
      <p className="eyebrow">404</p>
      <h1>That page drifted out to sea.</h1>
      <p>Head back to Conch or contact {contactEmail} if you expected something here.</p>
      <a className="button button-primary" href="/">Back to Conch</a>
    </main>
  );
}

function normalizePath(pathname) {
  if (!pathname || pathname === "/index.html") return "/";
  return pathname.endsWith("/") && pathname.length > 1 ? pathname.slice(0, -1) : pathname;
}

export default App;
