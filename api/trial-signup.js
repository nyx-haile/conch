import { readJsonBody, setJsonNoStoreHeaders } from "./_http.js";
import { createEmailConfirmation } from "./_session.js";

export default async function handler(request, response) {
  setJsonHeaders(response);

  if (request.method === "OPTIONS") {
    response.status(204).end();
    return;
  }

  if (request.method !== "POST") {
    response.status(405).json({ state: "error", message: "POST required." });
    return;
  }

  const body = await readJsonBody(request);

  try {
    const confirmation = createEmailConfirmation(body.email);
    const confirmationUrl = buildConfirmationUrl(request, confirmation.confirmationToken);
    const emailResult = await sendConfirmationEmail(confirmation.email, confirmationUrl);

    if (!emailResult.ok && !allowDevConfirmationLink()) {
      response.status(503).json({
        state: "email_config_missing",
        message: "Email confirmation is required before trials can start, but outbound email is not configured.",
      });
      return;
    }

    response.status(200).json({
      state: "email_confirmation_required",
      confirmationId: confirmation.confirmationId,
      email: confirmation.email,
      expiresAt: confirmation.expiresAt,
      devConfirmationUrl: allowDevConfirmationLink() ? confirmationUrl : undefined,
      message: "Check your email to confirm this free trial.",
    });
  } catch {
    response.status(400).json({ state: "signup_required", message: "A valid email is required." });
  }
}

async function sendConfirmationEmail(email, confirmationUrl) {
  const apiKey = process.env.RESEND_API_KEY?.trim();
  const from = process.env.CONCH_EMAIL_FROM?.trim();
  if (!apiKey || !from) return { ok: false };

  const response = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify({
      from,
      to: email,
      subject: "Confirm your Conch free trial",
      html: `<p>Confirm your Conch free trial:</p><p><a href="${escapeHtml(confirmationUrl)}">Open Conch</a></p><p>This link expires in 30 minutes.</p>`,
      text: `Confirm your Conch free trial: ${confirmationUrl}\n\nThis link expires in 30 minutes.`,
    }),
  });
  return { ok: response.ok };
}

function buildConfirmationUrl(request, token) {
  const configuredBase = process.env.CONCH_PUBLIC_APP_URL?.trim();
  const host = request.headers["x-forwarded-host"] || request.headers.host || "localhost:5173";
  const proto = request.headers["x-forwarded-proto"] || (String(host).startsWith("localhost") ? "http" : "https");
  const base = configuredBase || `${proto}://${host}`;
  return `${base.replace(/\/$/, "")}/app/confirm?token=${encodeURIComponent(token)}`;
}

function allowDevConfirmationLink() {
  return process.env.NODE_ENV !== "production" || process.env.CONCH_ALLOW_DEV_CONFIRMATION_LINK === "true";
}

function escapeHtml(value) {
  return String(value)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

function setJsonHeaders(response) {
  setJsonNoStoreHeaders(response, [["Access-Control-Allow-Headers", "Content-Type"]]);
}
