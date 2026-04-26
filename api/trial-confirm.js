import { readJsonBody, setJsonNoStoreHeaders } from "./_http.js";
import { confirmTrialEmail } from "./_session.js";
import { assertTrialUsageStoreReady, globalFreeTrialBudgetCents, perUserTrialBudgetCents } from "./_usage.js";

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
  const token = typeof body.token === "string" ? body.token.trim() : "";
  const usageReady = await assertTrialUsageStoreReady();
  if (!usageReady.ok) {
    response.status(usageReady.status).json(usageReady);
    return;
  }

  try {
    const session = confirmTrialEmail(token);
    response.status(200).json({
      state: "consent_required",
      ...session,
      trialBudgetCents: perUserTrialBudgetCents,
      globalFreeTrialBudgetCents,
      message: "Email confirmed. Continue to recording consent.",
    });
  } catch {
    response.status(400).json({ state: "email_confirmation_required", message: "The confirmation link is invalid or expired." });
  }
}

function setJsonHeaders(response) {
  setJsonNoStoreHeaders(response, [["Access-Control-Allow-Headers", "Content-Type"]]);
}
