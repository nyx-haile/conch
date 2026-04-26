import { createHmac, randomUUID, timingSafeEqual } from "node:crypto";
import { globalFreeTrialBudgetCents, perUserTrialBudgetCents } from "./_usage.js";

const trialDurationMs = 14 * 24 * 60 * 60 * 1000;
const confirmationDurationMs = 30 * 60 * 1000;
const fallbackDevSecret = "conch-local-dev-session-secret";

export function createEmailConfirmation(email, now = Date.now()) {
  const normalizedEmail = normalizeEmail(email);
  if (!normalizedEmail) {
    throw new Error("valid email required");
  }

  const payload = {
    type: "email_confirmation",
    confirmationId: `confirm_${randomUUID()}`,
    email: normalizedEmail,
    iat: Math.floor(now / 1000),
    exp: Math.floor((now + confirmationDurationMs) / 1000),
  };

  return {
    confirmationId: payload.confirmationId,
    email: payload.email,
    expiresAt: new Date(payload.exp * 1000).toISOString(),
    confirmationToken: signPayload(payload),
  };
}

export function confirmTrialEmail(confirmationToken, now = Date.now()) {
  const confirmation = verifySignedPayload(confirmationToken, now);
  if (
    !confirmation ||
    confirmation.type !== "email_confirmation" ||
    typeof confirmation.confirmationId !== "string" ||
    !confirmation.confirmationId.startsWith("confirm_") ||
    !normalizeEmail(confirmation.email)
  ) {
    throw new Error("valid confirmation required");
  }

  return createTrialSession(confirmation.email, now);
}

export function createTrialSession(email, now = Date.now()) {
  const normalizedEmail = normalizeEmail(email);
  if (!normalizedEmail) {
    throw new Error("valid email required");
  }

  const payload = {
    type: "trial_session",
    sessionId: `trial_${randomUUID()}`,
    email: normalizedEmail,
    emailConfirmed: true,
    trial: "active",
    trialBudgetCents: perUserTrialBudgetCents,
    globalFreeTrialBudgetCents,
    iat: Math.floor(now / 1000),
    exp: Math.floor((now + trialDurationMs) / 1000),
  };

  return {
    sessionId: payload.sessionId,
    email: payload.email,
    emailConfirmed: payload.emailConfirmed,
    trial: payload.trial,
    trialBudgetCents: payload.trialBudgetCents,
    globalFreeTrialBudgetCents: payload.globalFreeTrialBudgetCents,
    expiresAt: new Date(payload.exp * 1000).toISOString(),
    sessionToken: signPayload(payload),
  };
}

export function verifyTrialSessionToken(token, now = Date.now()) {
  const payload = verifySignedPayload(token, now);
  if (
    !payload ||
    payload.type !== "trial_session" ||
    typeof payload.sessionId !== "string" ||
    !payload.sessionId.startsWith("trial_") ||
    payload.trial !== "active" ||
    payload.emailConfirmed !== true ||
    !normalizeEmail(payload.email) ||
    payload.trialBudgetCents !== perUserTrialBudgetCents ||
    payload.globalFreeTrialBudgetCents !== globalFreeTrialBudgetCents
  ) {
    return null;
  }

  return payload;
}

export function extractBearerToken(request) {
  const auth = request.headers.authorization || request.headers.Authorization || "";
  if (typeof auth === "string" && auth.toLowerCase().startsWith("bearer ")) {
    return auth.slice(7).trim();
  }
  const headerToken = request.headers["x-conch-session-token"];
  return Array.isArray(headerToken) ? headerToken[0] : headerToken || "";
}

function verifySignedPayload(token, now) {
  if (typeof token !== "string" || token.length > 4096) {
    return null;
  }

  const [encodedPayload, encodedSignature] = token.split(".");
  if (!encodedPayload || !encodedSignature) {
    return null;
  }

  const expectedSignature = hmac(encodedPayload);
  if (!safeEqual(encodedSignature, expectedSignature)) {
    return null;
  }

  const payload = parsePayload(encodedPayload);
  if (!payload || typeof payload.exp !== "number" || payload.exp <= Math.floor(now / 1000)) {
    return null;
  }

  return payload;
}

function signPayload(payload) {
  const encodedPayload = base64UrlEncode(JSON.stringify(payload));
  return `${encodedPayload}.${hmac(encodedPayload)}`;
}

function hmac(value) {
  return createHmac("sha256", sessionSecret()).update(value).digest("base64url");
}

function sessionSecret() {
  return (
    process.env.CONCH_SESSION_SIGNING_SECRET?.trim() ||
    process.env.DEEPGRAM_API_KEY?.trim() ||
    fallbackDevSecret
  );
}

function normalizeEmail(email) {
  return typeof email === "string" && email.includes("@") ? email.trim().toLowerCase() : "";
}

function parsePayload(encodedPayload) {
  try {
    return JSON.parse(Buffer.from(encodedPayload, "base64url").toString("utf8"));
  } catch {
    return null;
  }
}

function base64UrlEncode(value) {
  return Buffer.from(value, "utf8").toString("base64url");
}

function safeEqual(left, right) {
  try {
    const leftBuffer = Buffer.from(left);
    const rightBuffer = Buffer.from(right);
    return leftBuffer.length === rightBuffer.length && timingSafeEqual(leftBuffer, rightBuffer);
  } catch {
    return false;
  }
}
