interface Env {
  ELY_CLIENT_ID: string;
  ELY_CLIENT_SECRET: string;
  ELY_REDIRECT_URI: string;
}

const TOKEN_ENDPOINT = "https://account.ely.by/api/oauth2/v1/token";
const MAX_BODY_BYTES = 8192;
const MAX_TOKEN_LENGTH = 4096;

function json(body: unknown, status = 200): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "no-store, max-age=0",
      "X-Content-Type-Options": "nosniff",
    },
  });
}

async function readBody(request: Request): Promise<string | null> {
  const reader = request.body?.getReader();
  if (!reader) return null;
  const chunks: Uint8Array[] = [];
  let size = 0;
  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    size += value.byteLength;
    if (size > MAX_BODY_BYTES) {
      await reader.cancel();
      return null;
    }
    chunks.push(value);
  }
  const bytes = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) {
    bytes.set(chunk, offset);
    offset += chunk.byteLength;
  }
  return new TextDecoder().decode(bytes);
}

function validToken(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= MAX_TOKEN_LENGTH;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname !== "/v1/token") return json({ error: "not_found" }, 404);
    if (request.method !== "POST") return json({ error: "method_not_allowed" }, 405);
    if (!request.headers.get("content-type")?.toLowerCase().startsWith("application/json")) {
      return json({ error: "invalid_request" }, 400);
    }
    if (!env.ELY_CLIENT_ID || !env.ELY_CLIENT_SECRET || !env.ELY_REDIRECT_URI) {
      return json({ error: "service_unavailable" }, 503);
    }

    const text = await readBody(request);
    if (text === null) return json({ error: "invalid_request" }, 400);

    let body: Record<string, unknown>;
    try {
      const parsed: unknown = JSON.parse(text);
      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        return json({ error: "invalid_request" }, 400);
      }
      body = parsed as Record<string, unknown>;
    } catch {
      return json({ error: "invalid_request" }, 400);
    }

    const form = new URLSearchParams({
      client_id: env.ELY_CLIENT_ID,
      client_secret: env.ELY_CLIENT_SECRET,
    });
    if (body.grant_type === "authorization_code" && validToken(body.code)) {
      form.set("grant_type", "authorization_code");
      form.set("redirect_uri", env.ELY_REDIRECT_URI);
      form.set("code", body.code);
    } else if (body.grant_type === "refresh_token" && validToken(body.refresh_token)) {
      form.set("grant_type", "refresh_token");
      form.set("scope", "account_info minecraft_server_session offline_access");
      form.set("refresh_token", body.refresh_token);
    } else {
      return json({ error: "invalid_request" }, 400);
    }

    let upstream: Response;
    try {
      upstream = await fetch(TOKEN_ENDPOINT, {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
          Accept: "application/json",
        },
        body: form,
        redirect: "manual",
      });
    } catch {
      return json({ error: "upstream_unavailable" }, 502);
    }
    if (!upstream.ok) {
      let error = "authorization_failed";
      try {
        const details: unknown = await upstream.json();
        if (typeof details === "object" && details !== null && !Array.isArray(details)) {
          const code = (details as Record<string, unknown>).error;
          if (
            typeof code === "string" &&
            [
              "invalid_client",
              "invalid_grant",
              "invalid_request",
              "invalid_scope",
              "unsupported_grant_type",
              "unauthorized_client",
            ].includes(code)
          ) {
            error = code;
          }
        }
      } catch {}
      if (upstream.status === 400 || upstream.status === 401) {
        return json({ error }, 401);
      }
      if (error !== "authorization_failed") return json({ error }, 401);
      return json({ error: "upstream_unavailable", upstream_status: upstream.status }, 502);
    }

    let token: Record<string, unknown>;
    try {
      const parsed: unknown = await upstream.json();
      if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
        return json({ error: "upstream_invalid_response" }, 502);
      }
      token = parsed as Record<string, unknown>;
    } catch {
      return json({ error: "upstream_invalid_response" }, 502);
    }
    if (!validToken(token.access_token)) return json({ error: "upstream_invalid_response" }, 502);

    const result: Record<string, unknown> = {
      access_token: token.access_token,
      expires_in: typeof token.expires_in === "number" && Number.isFinite(token.expires_in) ? token.expires_in : 86400,
    };
    if (validToken(token.refresh_token)) result.refresh_token = token.refresh_token;
    if (typeof token.token_type === "string") result.token_type = token.token_type;
    return json(result);
  },
};
