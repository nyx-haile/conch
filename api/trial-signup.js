import { readJsonBody, setJsonNoStoreHeaders } from "./_http.js";
import { requestSupabaseEmailConfirmation } from "./_supabase.js";

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
  const email = normalizeEmail(body.email);
  if (!email) {
    response.status(400).json({ state: "signup_required", message: "A valid email is required." });
    return;
  }

  const result = await requestSupabaseEmailConfirmation(email, confirmationRedirectUrl(request));
  if (!result.ok) {
    response.status(result.status).json(result);
    return;
  }

  response.status(200).json({
    state: "email_confirmation_required",
    email,
    message: "Check your email for a Supabase confirmation link to activate this free trial.",
  });
}

function confirmationRedirectUrl(request) {
  const configuredBase = process.env.CONCH_PUBLIC_APP_URL?.trim();
  const host = request.headers["x-forwarded-host"] || request.headers.host || "localhost:5173";
  const proto = request.headers["x-forwarded-proto"] || (String(host).startsWith("localhost") ? "http" : "https");
  const base = configuredBase || `${proto}://${host}`;
  return `${base.replace(/\/$/, "")}/app/confirm`;
}

function normalizeEmail(email) {
  return typeof email === "string" && email.includes("@") ? email.trim().toLowerCase() : "";
}

function setJsonHeaders(response) {
  setJsonNoStoreHeaders(response, [["Access-Control-Allow-Headers", "Content-Type"]]);
}
