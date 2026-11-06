use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub const CLIENT_ID: &str = "horizon-ely1";
pub const REDIRECT_URI: &str = "https://justasocial.github.io/horizon-elyby/ely-callback/";
const SCOPE: &str = "account_info minecraft_server_session offline_access";
const AUTHORIZATION: &str = "https://account.ely.by/oauth2/v1";
const TOKEN: &str = "https://account.ely.by/api/oauth2/v1/token";
const USER_INFO: &str = "https://account.ely.by/api/account/v1/info";
const ORIGIN: &str = "https://justasocial.github.io";
const RECEIVE_TIMEOUT: Duration = Duration::from_secs(900);

pub struct Pending {
    pub url: String,
    listener: TcpListener,
    state: String,
}

pub fn client_id() -> String {
    std::env::var("HORIZON_ELY_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| CLIENT_ID.to_owned())
}

pub fn configured() -> bool {
    client_secret().is_ok()
}

fn client_secret() -> Result<String, String> {
    let environment_secret = std::env::var("HORIZON_ELY_CLIENT_SECRET").ok();
    let file_secret = dirs::config_dir()
        .map(|directory| {
            directory
                .join("Horizon Revo")
                .join("ely-by-client-secret.txt")
        })
        .and_then(|path| std::fs::read_to_string(path).ok());
    environment_secret
        .or(file_secret)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "Ely.by client secret is missing. Set HORIZON_ELY_CLIENT_SECRET or save it in %APPDATA%\\Horizon Revo\\ely-by-client-secret.txt, then restart Horizon Revo.".into()
        })
}

impl Pending {
    pub fn begin() -> Result<Self, String> {
        client_secret()?;
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
            .append_pair("scope", SCOPE)
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
        respond(stream, 200, "Authorization was not completed", origin);
        return Err("Ely.by sign-in was cancelled or denied.".into());
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
    let client_id = client_id();
    let client_secret = client_secret()?;
    let (status, token) = send(
        request_to("POST", TOKEN),
        Some(Body::Form(vec![
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("redirect_uri", REDIRECT_URI),
            ("grant_type", "authorization_code"),
            ("code", code),
        ])),
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
    let client_id = client_id();
    let client_secret = client_secret()?;
    let (status, token) = send(
        request_to("POST", TOKEN),
        Some(Body::Form(vec![
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("scope", SCOPE),
            ("refresh_token", &secrets.refresh_token),
            ("grant_type", "refresh_token"),
        ])),
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
