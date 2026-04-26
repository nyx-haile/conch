import { createHmac, randomUUID, timingSafeEqual } from "node:crypto";

const trialDurationMs = 14 * 24 * 60 * 60 * 1000;
const fallbackDevSecret = "conch-local-dev-session-secret";

export function createTrialSession(email, now = Date.now()) {
  const normalizedEmail = normalizeEmail(email);
  if (!normalizedEmail) {
    throw new Error("valid email required");
  }

  const payload = {
    sessionId: `trial_${randomUUID()}`,
    email: normalizedEmail,
    trial: "active",
    iat: Math.floor(now / 1000),
    exp: Math.floor((now + trialDurationMs) / 1000),
  };

  return {
    sessionId: payload.sessionId,
    email: payload.email,
    trial: payload.trial,
    expiresAt: new Date(payload.exp * 1000).toISOString(),
    sessionToken: signTrialSession(payload),
  };
}

export function verifyTrialSessionToken(token, now = Date.now()) {
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
  if (!payload) {
    return null;
  }

  if (
    typeof payload.sessionId !== "string" ||
    !payload.sessionId.startsWith("trial_") ||
    payload.trial !== "active" ||
    typeof payload.exp !== "number" ||
    payload.exp <= Math.floor(now / 1000)
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

function signTrialSession(payload) {
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
