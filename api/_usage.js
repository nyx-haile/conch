import { callSupabaseRpc, supabaseConfigMissing, supabaseConfigured } from "./_supabase.js";

export const perUserTrialBudgetCents = Number.parseInt(process.env.CONCH_TRIAL_USER_BUDGET_CENTS || "1000", 10);
export const globalFreeTrialBudgetCents = Number.parseInt(process.env.CONCH_FREE_TRIAL_GLOBAL_BUDGET_CENTS || "1000000", 10);
export const tokenGrantReservationCents = Number.parseInt(process.env.CONCH_TRIAL_TOKEN_RESERVATION_CENTS || "1000", 10);

export async function reserveTrialTokenGrant(session) {
  return reserveTrialUsage(session, tokenGrantReservationCents);
}

export async function ensureTrialUsageAccount(user) {
  if (!supabaseConfigured({ service: true })) return usageConfigMissing();
  const payload = await callSupabaseRpc("conch_ensure_trial_account", {
    p_user_id: user.id,
    p_email: user.email,
    p_per_user_budget_cents: perUserTrialBudgetCents,
    p_global_budget_cents: globalFreeTrialBudgetCents,
  });
  return normalizeUsagePayload(payload);
}

async function reserveTrialUsage(session, cents) {
  if (!supabaseConfigured({ service: true })) return usageConfigMissing();
  if (!session?.supabaseUserId || !session?.email) {
    return {
      ok: false,
      state: "signup_required",
      status: 401,
      message: "Supabase-confirmed trial session required.",
      perUserBudgetCents,
      globalFreeTrialBudgetCents,
    };
  }

  const payload = await callSupabaseRpc("conch_reserve_trial_usage", {
    p_user_id: session.supabaseUserId,
    p_email: session.email,
    p_reservation_cents: cents,
    p_per_user_budget_cents: perUserTrialBudgetCents,
    p_global_budget_cents: globalFreeTrialBudgetCents,
  });
  return normalizeUsagePayload(payload);
}

function normalizeUsagePayload(payload) {
  if (!payload?.ok) {
    return {
      ...usageConfigMissing(),
      ...(payload || {}),
      ok: false,
      status: payload?.status || payload?.status_code || payload?.http_status || 503,
    };
  }

  return {
    ok: true,
    usageEnforced: true,
    reservationCents: payload.reservation_cents || payload.reservationCents || 0,
    perUserUsedCents: payload.per_user_used_cents || payload.perUserUsedCents || 0,
    perUserBudgetCents: payload.per_user_budget_cents || payload.perUserBudgetCents || perUserTrialBudgetCents,
    globalUsedCents: payload.global_used_cents || payload.globalUsedCents || 0,
    globalFreeTrialBudgetCents: payload.global_budget_cents || payload.globalFreeTrialBudgetCents || globalFreeTrialBudgetCents,
  };
}

function usageConfigMissing() {
  const missing = supabaseConfigMissing();
  return {
    ...missing,
    state: "usage_config_missing",
    message: "Supabase Postgres trial usage tables and RPC functions must be configured before voice can start.",
    perUserBudgetCents,
    globalFreeTrialBudgetCents,
  };
}
