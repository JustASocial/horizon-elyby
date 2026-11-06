use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub const CLIENT_ID: &str = "horizon-ely1";
pub const REDIRECT_URI: &str = "https://justasocial.github.io/horizon-elyby/ely-callback/";
const AUTHORIZATION: &str = "https://account.ely.by/oauth2/v1";
const USER_INFO: &str = "https://account.ely.by/api/account/v1/info";
const ORIGIN: &str = "https://justasocial.github.io";
const RECEIVE_TIMEOUT: Duration = Duration::from_secs(900);
const ELY_SCOPE: &str = "account_info minecraft_server_session offline_access";

pub fn broker_url() -> Option<&'static str> {
    option_env!("HORIZON_ELY_TOKEN_BROKER")
        .map(str::trim)
        .filter(|value| value.starts_with("https://") && value.ends_with("/v1/token"))
}

pub struct Pending {
    pub url: String,
    listener: TcpListener,
    state: String,
}

pub fn client_id() -> String {
    CLIENT_ID.to_owned()
}

pub fn configured() -> bool {
    broker_url().is_some()
}

impl Pending {
    pub fn begin() -> Result<Self, String> {
        if !configured() {
            return Err("Ely.by sign-in is not available in this build.".into());
        }
        let listener = (0..32)
            .find_map(|_| {
                let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
                let port = listener.local_addr().ok()?.port();
                (49152..=65535).contains(&port).then_some(listener)
            })
            .ok_or("Could not open a local Ely.by sign-in callback. Try again.")?;
        listener
            .set_nonblocking(true)
            .map_err(|_| "Could not prepare the local Ely.by sign-in callback.")?;
        let port = listener
            .local_addr()
            .map_err(|_| "Could not read the local Ely.by sign-in address.")?
            .port();
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| "Could not secure the Ely.by sign-in request.")?;
        let nonce = nonce
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let state = format!("horizon-revo-ely:{port}:{nonce}");
        let mut url =
            url::Url::parse(AUTHORIZATION).map_err(|_| "Could not prepare Ely.by sign-in.")?;
        url.query_pairs_mut()
            .append_pair("client_id", &client_id())
            .append_pair("redirect_uri", REDIRECT_URI)
            .append_pair("response_type", "code")
            .append_pair("scope", ELY_SCOPE)
            .append_pair("state", &state);
        Ok(Self {
            url: url.into(),
            listener,
            state,
        })
    }

    pub fn wait(self, cancelled: &AtomicBool) -> Result<String, String> {
        let deadline = Instant::now() + RECEIVE_TIMEOUT;
        loop {
            if cancelled.load(Ordering::Relaxed) {
                return Err("Sign-in cancelled.".into());
            }
            if Instant::now() >= deadline {
                return Err("Ely.by sign-in expired. Start again.".into());
            }
            match self.listener.accept() {
                Ok((mut stream, _)) => match callback(&mut stream, &self.state) {
                    Ok(Some(code)) => return Ok(code),
                    Ok(None) => {}
                    Err(error) => return Err(error),
                },
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => return Err("Could not receive the Ely.by sign-in response.".into()),
            }
        }
    }
}

fn callback(stream: &mut TcpStream, expected_state: &str) -> Result<Option<String>, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|_| "Could not read the Ely.by response.")?;
    let mut request = Vec::with_capacity(2048);
    let mut chunk = [0u8; 1024];
    while request.len() < 8192 && !request.windows(4).any(|window| window == b"\r\n\r\n") {
        let count = stream
            .read(&mut chunk)
            .map_err(|_| "Could not read the Ely.by response.")?;
        if count == 0 {
            break;
        }
        request.extend_from_slice(&chunk[..count]);
    }
    let text = String::from_utf8_lossy(&request);
    let mut lines = text.split("\r\n");
    let mut first = lines.next().unwrap_or_default().split_whitespace();
    let method = first.next().unwrap_or_default();
    let path = first.next().unwrap_or_default();
    let headers: Vec<_> = lines
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    let origin = headers
        .iter()
        .find(|(name, _)| name == "origin")
        .map(|(_, value)| value.as_str());
    if origin.is_some_and(|value| value != ORIGIN) {
        respond(stream, 403, "Forbidden", origin);
        return Ok(None);
    }
    if method == "OPTIONS" && path == "/oauth/ely/callback" {
        respond(stream, 204, "", origin);
        return Ok(None);
    }
    let Ok(url) = url::Url::parse(&format!("http://127.0.0.1{path}")) else {
        respond(stream, 400, "Invalid callback", origin);
        return Ok(None);
    };
    if method != "GET" || url.path() != "/oauth/ely/callback" {
        respond(stream, 404, "Not found", origin);
        return Ok(None);
    }
    let values: std::collections::HashMap<String, String> =
        url.query_pairs().into_owned().collect();
    if values.get("state").map(String::as_str) != Some(expected_state) {
        respond(stream, 403, "Sign-in state did not match", origin);
        return Ok(None);
    }
    if values.contains_key("error") {
        let error = values
            .get("error")
            .map(|value| value.chars().take(128).collect::<String>())
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown_error");
        let description = values
            .get("error_message")
            .map(|value| value.chars().take(256).collect::<String>())
            .filter(|value| !value.is_empty());
        let message = description.map_or_else(
            || format!("Ely.by authorization failed: {error}."),
            |description| format!("Ely.by authorization failed: {error}: {description}"),
        );
        respond(stream, 400, &message, origin);
        return Err(message);
    }
    let Some(code) = values
        .get("code")
        .filter(|value| !value.is_empty() && value.len() <= 4096)
    else {
        respond(stream, 400, "Missing authorization code", origin);
        return Ok(None);
    };
    respond(stream, 200, "Authorization received", origin);
    Ok(Some(code.clone()))
}

fn respond(stream: &mut TcpStream, status: u16, body: &str, origin: Option<&str>) {
    let reason = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        _ => "Not Found",
    };
    let cors = origin.filter(|value| *value == ORIGIN).map_or_else(String::new, |_| {
        format!(
            "Access-Control-Allow-Origin: {ORIGIN}\r\nAccess-Control-Allow-Private-Network: true\r\nVary: Origin\r\n"
        )
    });
    let body = if status == 204 { "" } else { body };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nAccess-Control-Allow-Methods: GET, OPTIONS\r\nAccess-Control-Allow-Headers: *\r\n{cors}\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

pub fn finish(code: &str) -> Result<(Account, Secrets), String> {
    let broker = broker_url().ok_or("Ely.by sign-in is not available in this build.")?;
    let (status, token) = send(
        request_to("POST", broker),
        Some(Body::Json(
            json!({ "grant_type": "authorization_code", "code": code }),
        )),
        "Ely.by",
    )?;
    if status != 200 {
        return Err("Ely.by refused the authorization code. Check the registered redirect URI and try again.".into());
    }
    let access_token = str_field(&token, "access_token")
        .filter(|value| !value.is_empty())
        .ok_or("Ely.by did not return an access token.")?;
    let refresh_token = str_field(&token, "refresh_token")
        .filter(|value| !value.is_empty())
        .ok_or("Ely.by did not return a refresh token. Check that offline_access is enabled.")?;
    let expires_in = token
        .get("expires_in")
        .and_then(Value::as_u64)
        .unwrap_or(86400);
    let (status, info) = send(
        bearer(request_to("GET", USER_INFO), &access_token),
        None,
        "Ely.by",
    )?;
    if status != 200 {
        return Err("Ely.by could not return the account profile. Try again.".into());
    }
    let uuid = str_field(&info, "uuid")
        .map(|value| value.replace('-', "").to_ascii_lowercase())
        .filter(|value| value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or("Ely.by returned an invalid profile UUID.")?;
    let name = str_field(&info, "username")
        .filter(|value| !value.is_empty() && value.chars().count() <= 16)
        .ok_or("Ely.by returned an invalid profile name.")?;
    Ok((
        Account {
            uuid,
            name,
            provider: Provider::Ely,
            skin: None,
            cape: None,
        },
        Secrets {
            refresh_token,
            sign_in_required: false,
            minecraft_token: access_token,
            minecraft_expires_at: now() + expires_in,
            xuid: None,
            owns: false,
        },
    ))
}

pub(super) fn refresh(secrets: &mut Secrets) -> Result<(), String> {
    if secrets.minecraft_expires_at > now() + RENEW_BEFORE {
        return Ok(());
    }
    let broker = broker_url().ok_or("Ely.by sign-in is not available in this build.")?;
    let (status, token) = send(
        request_to("POST", broker),
        Some(Body::Json(json!({
            "grant_type": "refresh_token",
            "refresh_token": secrets.refresh_token,
        }))),
        "Ely.by",
    )?;
    if status != 200 {
        secrets.sign_in_required = true;
        return Err("Ely.by sign-in expired. Sign in again.".into());
    }
    secrets.minecraft_token = str_field(&token, "access_token")
        .filter(|value| !value.is_empty())
        .ok_or("Ely.by did not return a refreshed access token.")?;
    if let Some(refresh) = str_field(&token, "refresh_token") {
        secrets.refresh_token = refresh;
    }
    secrets.minecraft_expires_at = now()
        + token
            .get("expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(86400);
    secrets.sign_in_required = false;
    Ok(())
}
