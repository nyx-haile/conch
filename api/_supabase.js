export function supabaseConfigured({ service = false } = {}) {
  return Boolean(supabaseUrl() && supabaseAnonKey() && (!service || supabaseServiceRoleKey()));
}

export function supabaseConfigMissing() {
  return {
    ok: false,
    state: "auth_config_missing",
    status: 503,
    message: "Supabase Auth and Postgres must be configured before free trials can start.",
  };
}

export async function requestSupabaseEmailConfirmation(email, redirectTo) {
  if (!supabaseConfigured()) return supabaseConfigMissing();

  const response = await fetch(`${supabaseUrl()}/auth/v1/otp`, {
    method: "POST",
    headers: supabaseHeaders(),
    body: JSON.stringify({
      email,
      create_user: true,
      options: { email_redirect_to: redirectTo },
    }),
  });

  if (!response.ok) {
    const payload = await response.json().catch(() => ({}));
    return {
      ok: false,
      state: "auth_config_missing",
      status: response.status >= 500 ? 503 : 400,
      message: payload.msg || payload.message || "Supabase could not send the confirmation email.",
    };
  }

  return { ok: true };
}

export async function getSupabaseUserFromAccessToken(accessToken) {
  if (!supabaseConfigured()) return supabaseConfigMissing();
  if (!accessToken) {
    return { ok: false, state: "email_confirmation_required", status: 400, message: "Missing Supabase access token." };
  }

  const response = await fetch(`${supabaseUrl()}/auth/v1/user`, {
    headers: supabaseHeaders({ accessToken }),
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || !payload.id) {
    return {
      ok: false,
      state: "email_confirmation_required",
      status: 400,
      message: payload.msg || payload.message || "Supabase could not verify this confirmed email session.",
    };
  }

  return { ok: true, user: payload, accessToken };
}

export async function verifySupabaseOtp(tokenHash, type = "email") {
  if (!supabaseConfigured()) return supabaseConfigMissing();
  if (!tokenHash) {
    return { ok: false, state: "email_confirmation_required", status: 400, message: "Missing Supabase confirmation token." };
  }

  const response = await fetch(`${supabaseUrl()}/auth/v1/verify`, {
    method: "POST",
    headers: supabaseHeaders(),
    body: JSON.stringify({ token_hash: tokenHash, type }),
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || !payload.user) {
    return {
      ok: false,
      state: "email_confirmation_required",
      status: 400,
      message: payload.msg || payload.message || "Supabase could not verify this confirmation link.",
    };
  }

  return { ok: true, user: payload.user, accessToken: payload.access_token };
}

export async function callSupabaseRpc(functionName, body) {
  if (!supabaseConfigured({ service: true })) return supabaseConfigMissing();

  const response = await fetch(`${supabaseUrl()}/rest/v1/rpc/${functionName}`, {
    method: "POST",
    headers: supabaseHeaders({ service: true }),
    body: JSON.stringify(body),
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok) {
    return {
      ok: false,
      state: "usage_config_missing",
      status: 503,
      message: payload.message || `Supabase RPC ${functionName} is not configured.`,
      details: payload.details,
    };
  }

  return Array.isArray(payload) ? payload[0] : payload;
}

function supabaseHeaders({ accessToken = "", service = false, prefer = "" } = {}) {
  const key = service ? supabaseServiceRoleKey() : supabaseAnonKey();
  const headers = {
    apikey: key,
    Authorization: `Bearer ${accessToken || key}`,
    "Content-Type": "application/json",
    Accept: "application/json",
  };
  if (prefer) headers.Prefer = prefer;
  return headers;
}

function supabaseUrl() {
  return (process.env.SUPABASE_URL || process.env.NEXT_PUBLIC_SUPABASE_URL || "").replace(/\/$/, "");
}

function supabaseAnonKey() {
  return process.env.SUPABASE_ANON_KEY || process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY || "";
}

function supabaseServiceRoleKey() {
  return process.env.SUPABASE_SERVICE_ROLE_KEY || "";
}
