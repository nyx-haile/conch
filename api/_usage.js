import { createHash } from "node:crypto";

export const perUserTrialBudgetCents = Number.parseInt(process.env.CONCH_TRIAL_USER_BUDGET_CENTS || "1000", 10);
export const globalFreeTrialBudgetCents = Number.parseInt(process.env.CONCH_FREE_TRIAL_GLOBAL_BUDGET_CENTS || "1000000", 10);
export const tokenGrantReservationCents = Number.parseInt(process.env.CONCH_TRIAL_TOKEN_RESERVATION_CENTS || "100", 10);

const namespace = process.env.CONCH_USAGE_NAMESPACE || "conch:trial";

export async function reserveTrialTokenGrant(email) {
  return reserveTrialUsage(email, tokenGrantReservationCents);
}

export async function assertTrialUsageStoreReady() {
  if (usageStoreConfigured() || allowUnmeteredDevTrials()) {
    return { ok: true };
  }
  return usageConfigMissing();
}

async function reserveTrialUsage(email, cents) {
  if (!usageStoreConfigured()) {
    if (allowUnmeteredDevTrials()) {
      return {
        ok: true,
        usageEnforced: false,
        reservationCents: 0,
        perUserBudgetCents,
        globalFreeTrialBudgetCents,
      };
    }
    return usageConfigMissing();
  }

  const userKey = `${namespace}:user:${hashEmail(email)}:cents`;
  const globalKey = `${namespace}:global:cents`;
  const [userUsed, globalUsed] = await Promise.all([redisNumber("GET", userKey), redisNumber("GET", globalKey)]);

  if (userUsed + cents > perUserTrialBudgetCents) {
    return {
      ok: false,
      state: "usage_limit_reached",
      status: 402,
      message: "This free-trial email has reached its $10 managed-usage budget.",
      perUserUsedCents: userUsed,
      perUserBudgetCents,
      globalUsedCents: globalUsed,
      globalFreeTrialBudgetCents,
    };
  }

  if (globalUsed + cents > globalFreeTrialBudgetCents) {
    return {
      ok: false,
      state: "free_trials_closed",
      status: 403,
      message: "Free trials are temporarily closed after the managed-usage pool reached $10,000.",
      perUserUsedCents: userUsed,
      perUserBudgetCents,
      globalUsedCents: globalUsed,
      globalFreeTrialBudgetCents,
    };
  }

  const [nextUser, nextGlobal] = await Promise.all([redisNumber("INCRBY", userKey, cents), redisNumber("INCRBY", globalKey, cents)]);
  return {
    ok: true,
    usageEnforced: true,
    reservationCents: cents,
    perUserUsedCents: nextUser,
    perUserBudgetCents,
    globalUsedCents: nextGlobal,
    globalFreeTrialBudgetCents,
  };
}

function usageStoreConfigured() {
  return Boolean(redisUrl() && redisToken());
}

function allowUnmeteredDevTrials() {
  return process.env.NODE_ENV !== "production" || process.env.CONCH_ALLOW_UNMETERED_DEV_TRIALS === "true";
}

function usageConfigMissing() {
  return {
    ok: false,
    state: "usage_config_missing",
    status: 503,
    message: "Free trials need server-side usage metering before voice can start.",
    perUserBudgetCents,
    globalFreeTrialBudgetCents,
  };
}

async function redisNumber(command, ...args) {
  const response = await fetch(`${redisUrl()}/${[command, ...args].map(encodeURIComponent).join("/")}`, {
    headers: { Authorization: `Bearer ${redisToken()}` },
  });
  const payload = await response.json().catch(() => ({}));
  if (!response.ok || payload.error) {
    throw new Error(payload.error || `Usage store ${command} failed.`);
  }
  return Number.parseInt(payload.result || "0", 10) || 0;
}

function redisUrl() {
  return (process.env.KV_REST_API_URL || process.env.UPSTASH_REDIS_REST_URL || "").replace(/\/$/, "");
}

function redisToken() {
  return process.env.KV_REST_API_TOKEN || process.env.UPSTASH_REDIS_REST_TOKEN || "";
}

function hashEmail(email) {
  return createHash("sha256").update(String(email).trim().toLowerCase()).digest("hex");
}
