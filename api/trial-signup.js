import { readJsonBody, setJsonNoStoreHeaders } from "./_http.js";
import { createTrialSession } from "./_session.js";

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
    const session = createTrialSession(body.email);
    response.status(200).json({
      state: "consent_required",
      ...session,
      message: "Free trial active. Continue to recording consent.",
    });
  } catch {
    response.status(400).json({ state: "signup_required", message: "A valid email is required." });
  }
}

function setJsonHeaders(response) {
  setJsonNoStoreHeaders(response, [["Access-Control-Allow-Headers", "Content-Type"]]);
}
