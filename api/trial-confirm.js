import { readJsonBody, setJsonNoStoreHeaders } from "./_http.js";
import { createTrialSession } from "./_session.js";
import { getSupabaseUserFromAccessToken, verifySupabaseOtp } from "./_supabase.js";
import { ensureTrialUsageAccount, globalFreeTrialBudgetCents, perUserTrialBudgetCents } from "./_usage.js";

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
  const authResult = body.accessToken
    ? await getSupabaseUserFromAccessToken(String(body.accessToken))
    : await verifySupabaseOtp(String(body.tokenHash || ""), String(body.type || "email"));

  if (!authResult.ok) {
    response.status(authResult.status).json(authResult);
    return;
  }

  const user = authResult.user;
  if (!user.email || !(user.email_confirmed_at || user.confirmed_at || authResult.accessToken)) {
    response.status(401).json({
      state: "email_confirmation_required",
      message: "Confirm your email with Supabase before starting a free trial.",
    });
    return;
  }

  const usageAccount = await ensureTrialUsageAccount(user);
  if (!usageAccount.ok) {
    response.status(usageAccount.status).json(usageAccount);
    return;
  }

  try {
    const session = createTrialSession(user, usageAccount);
    response.status(200).json({
      state: "consent_required",
      ...session,
      trialBudgetCents: usageAccount.perUserBudgetCents || perUserTrialBudgetCents,
      globalFreeTrialBudgetCents: usageAccount.globalFreeTrialBudgetCents || globalFreeTrialBudgetCents,
      message: "Supabase email confirmed. Continue to recording consent.",
    });
  } catch {
    response.status(400).json({ state: "email_confirmation_required", message: "The Supabase confirmation session is invalid." });
  }
}

function setJsonHeaders(response) {
  setJsonNoStoreHeaders(response, [["Access-Control-Allow-Headers", "Content-Type"]]);
}
