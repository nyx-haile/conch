import { readJsonBody, setJsonNoStoreHeaders } from "./_http.js";
import { extractBearerToken, verifyTrialSessionToken } from "./_session.js";

const deepgramGrantUrl = "https://api.deepgram.com/v1/auth/grant";
const tokenTtlSeconds = 60;

export default async function handler(request, response) {
  setSecurityHeaders(response);

  if (request.method === "OPTIONS") {
    response.status(204).end();
    return;
  }

  if (request.method !== "POST") {
    response.status(405).json({ state: "error", message: "POST required." });
    return;
  }

  const body = await readJsonBody(request);
  const session = verifyTrialSessionToken(extractBearerToken(request));
  const requestedSessionId = firstString(request.headers["x-conch-session"], body.sessionId);
  const consent = firstString(body.consentVersion, body.recordingConsentVersion);

  if (!session || (requestedSessionId && requestedSessionId !== session.sessionId)) {
    response.status(401).json({
      state: "signup_required",
      message: "Server-issued free-trial session required.",
    });
    return;
  }

  if (!consent) {
    response.status(428).json({
      state: "consent_required",
      message: "Recording consent is required before voice starts.",
    });
    return;
  }

  const serverKey = process.env.DEEPGRAM_API_KEY?.trim();
  if (!serverKey) {
    response.status(503).json({
      state: "voice_config_missing",
      provider: "deepgram",
      message: "Deepgram voice is not configured on this deployment yet.",
    });
    return;
  }

  try {
    const grantResponse = await fetch(deepgramGrantUrl, {
      method: "POST",
      headers: {
        Authorization: `Token ${serverKey}`,
        "Content-Type": "application/json",
        Accept: "application/json",
      },
      body: JSON.stringify({ ttl_seconds: tokenTtlSeconds }),
    });
    const payload = await grantResponse.json().catch(() => ({}));

    if (!grantResponse.ok || !payload.access_token) {
      const forbidden = grantResponse.status === 403;
      response.status(forbidden ? 503 : 502).json({
        state: forbidden ? "voice_config_missing" : "error",
        provider: "deepgram",
        message: forbidden
          ? "Deepgram key cannot mint browser tokens. Use a key with token-grant permissions."
          : "Deepgram token grant failed.",
      });
      return;
    }

    response.status(200).json({
      state: "ready_to_talk",
      provider: "deepgram",
      token_type: "Bearer",
      access_token: payload.access_token,
      expires_in: payload.expires_in ?? tokenTtlSeconds,
      websocket_url: "wss://api.deepgram.com/v1/listen",
      model: "nova-3",
    });
  } catch {
    response.status(502).json({
      state: "error",
      provider: "deepgram",
      message: "Deepgram token broker could not reach Deepgram.",
    });
  }
}

function setSecurityHeaders(response) {
  setJsonNoStoreHeaders(response, [
    ["Access-Control-Allow-Headers", "Content-Type, Authorization, X-Conch-Session, X-Conch-Session-Token"],
  ]);
}

function firstString(...values) {
  for (const value of values) {
    if (Array.isArray(value)) {
      const found = value.find((item) => typeof item === "string" && item.trim());
      if (found) return found.trim();
      continue;
    }
    if (typeof value === "string" && value.trim()) {
      return value.trim();
    }
  }
  return "";
}
