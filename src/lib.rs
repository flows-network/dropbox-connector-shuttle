use std::collections::HashMap;

use axum::{
    extract::{ContentLengthLimit, Json, Multipart, Query},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Extension, Router,
};
use lazy_static::lazy_static;
use mongodb::{bson::doc, Collection, Database};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use reqwest::{header, Client};
use rsa::{PaddingScheme, PublicKey, RsaPrivateKey, RsaPublicKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sync_wrapper::SyncWrapper;

const RSA_BITS: usize = 2048;

const HAIKU_API_PREFIX: &'static str = "https://wasmhaiku.com";

// If you deployed your connector, SERVICE_API_PREFIX should be set to https://<PROJECT_NAME>.shuttleapp.rs
const SERVICE_API_PREFIX: &'static str = "https://dropbox-connector-shuttle.shuttleapp.rs";

// You can find your app key and secret in the Dropbox App Console
const DROPBOX_APP_CLIENT_ID: &'static str = "zjw7qvenwttf5zf";
const DROPBOX_APP_CLIENT_SECRET: &'static str = "a27y9ftwanwsg10";

// The access token for WasmHaiku, which you can find it when you creating a connector
const HAIKU_AUTH_TOKEN: &'static str = "MDQ6VXNlcjM5MTk2MzAy";

// 32 bytes random string, but it must be CONSTANT otherwise it will NOT be able to decrypt the previously encrypted token
const RSA_RAND_SEED: [u8; 32] = *b"wWud6hFm7mcCj$^2eeffv2d@2aeLYNUn";

lazy_static! {
    static ref REDIRECT_URL: String = format!("{}/auth", SERVICE_API_PREFIX);
    static ref CHACHA8RNG: ChaCha8Rng = ChaCha8Rng::from_seed(RSA_RAND_SEED);
    static ref PRIVATE_KEY: RsaPrivateKey =
        RsaPrivateKey::new(&mut CHACHA8RNG.clone(), RSA_BITS).expect("failed to generate a key");
    static ref PUBLIC_KEY: RsaPublicKey = RsaPublicKey::from(&*PRIVATE_KEY);
    static ref HTTP_CLIENT: Client = Client::new();
}

fn encrypt(data: &str) -> String {
    hex::encode(
        PUBLIC_KEY
            .encrypt(
                &mut CHACHA8RNG.clone(),
                PaddingScheme::new_pkcs1v15_encrypt(),
                data.as_bytes(),
            )
            .expect("failed to encrypt"),
    )
}

fn decrypt<T: AsRef<[u8]>>(data: T) -> String {
    String::from_utf8(
        PRIVATE_KEY
            .decrypt(
                PaddingScheme::new_pkcs1v15_encrypt(),
                &hex::decode(data).unwrap(),
            )
            .expect("failed to decrypt"),
    )
    .unwrap()
}

async fn connect() -> impl IntoResponse {
    (StatusCode::FOUND, [(header::LOCATION, format!(
            "https://www.dropbox.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&token_access_type=offline",
            &*DROPBOX_APP_CLIENT_ID,
            urlencoding::encode(&*REDIRECT_URL)
        )
    )])
}

#[derive(Deserialize)]
struct AuthBody {
    code: String,
}

#[derive(Deserialize, Clone)]
struct AccessToken {
    access_token: String,
    refresh_token: Option<String>,
    account_id: Option<String>,
    // team_id: String,
}

enum AuthMode {
    Authorization(String),
    Refresh(String),
}

async fn get_access_token(mode: AuthMode) -> Result<AccessToken, String> {
    let params = match mode {
        AuthMode::Authorization(code) => [
            ("code", code),
            ("grant_type", "authorization_code".to_string()),
            ("redirect_uri", REDIRECT_URL.to_string()),
        ]
        .into_iter()
        .collect::<HashMap<&'static str, String>>(),

        AuthMode::Refresh(refresh_token) => [
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token".to_string()),
        ]
        .into_iter()
        .collect::<HashMap<&'static str, String>>(),
    };

    HTTP_CLIENT
        .post("https://api.dropbox.com/oauth2/token")
        .basic_auth(DROPBOX_APP_CLIENT_ID, Some(DROPBOX_APP_CLIENT_SECRET))
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?

        .json::<AccessToken>()
        .await
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct Name {
    display_name: String,
}

#[derive(Deserialize)]
struct Account {
    email: String,
    name: Name,
}

async fn get_account(at: &AccessToken) -> Result<Account, String> {
    HTTP_CLIENT
        .post("https://api.dropboxapi.com/2/users/get_current_account")
        .bearer_auth(at.access_token.clone())
        .send()
        .await
        .map_err(|e| e.to_string())?

        .json::<Account>()
        .await
        .map_err(|e| e.to_string())
}

async fn auth(req: Query<AuthBody>) -> impl IntoResponse {
    let at = match get_access_token(AuthMode::Authorization(req.code.clone())).await {
        Ok(at) => at,
        Err(e) => return Err((StatusCode::UNAUTHORIZED, e)),
    };

    let refresh_token = at.refresh_token
        .as_ref()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Missing refresh_token".to_string()))?;

    let id = at.account_id
        .as_ref()
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Missing account_id".to_string()))?;

    let account = get_account(&at)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
            format!("get_account failed: {}", e)))?;

    Ok((StatusCode::FOUND, [(header::LOCATION, format!(
        "{}/api/connected?authorId={}&authorName={}&authorState={}&refreshState={}",
        HAIKU_API_PREFIX,
        id,
        format!("{} ({})", account.name.display_name, account.email),
        encrypt(&at.access_token),
        encrypt(refresh_token)
    ))]))
}

#[derive(Deserialize)]
struct RefreshBody {
    refresh_state: String,
}

async fn refresh(req: Json<RefreshBody>) -> impl IntoResponse {
    get_access_token(AuthMode::Refresh(decrypt(&req.refresh_state)))
        .await
        .map(|at| (StatusCode::OK, Json(json!({
            "access_state": encrypt(&at.access_token),
            "refresh_state": req.refresh_state
        }))))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR,
            format!("get_access_token: {}", e)))
}

async fn upload(
    ContentLengthLimit(mut multipart): ContentLengthLimit<Multipart, { 150 * 1024 * 1024 }>,
) -> impl IntoResponse {
    let mut access_token = None;
    let mut file = Vec::new();
    let mut file_name = None;

    while let Some(field) = multipart.next_field().await.unwrap_or_else(|_| None) {
        match field.name().unwrap_or_default() {
            "file" => {
                file.append(
                    &mut field
                        .bytes()
                        .await
                        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                        .into(),
                );
            }
            "text" => {
                file_name = Some(
                    String::from_utf8(
                        field
                            .bytes()
                            .await
                            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
                            .to_vec(),
                    )
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?,
                );
            }
            "state" => {
                access_token = Some(decrypt(
                    field
                        .bytes()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
                ));
            }
            _ => {}
        }
    }

    if file.len() == 0 {
        return Err((StatusCode::BAD_REQUEST, "Invalid file".to_string()));
    }

    upload_file_to_dropbox(
        file,
        file_name.ok_or((StatusCode::BAD_REQUEST, "Missing file name".to_string()))?,
        access_token.ok_or((StatusCode::BAD_REQUEST, "Missing access_token".to_string()))?,
    )
    .await
    .map(|_| StatusCode::OK)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn upload_file_to_dropbox(
    file: Vec<u8>,
    file_name: String,
    access_token: String,
) -> Result<(), String> {
    let response = HTTP_CLIENT
        .post("https://content.dropboxapi.com/2/files/upload")
        .bearer_auth(access_token)
        .header(header::CONTENT_TYPE, "application/octet-stream")
        .header(
            "Dropbox-API-Arg",
            json!({
                "autorename": true,
                "path": file_name,
            })
            .to_string(),
        )
        .body(file)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    match response.status().is_success() {
        true => Ok(()),
        false => Err(format!(
            "Upload failed: {:?}",
            response.bytes().await.unwrap_or_default()
        )),
    }
}

#[derive(Deserialize)]
struct ChallengeBody {
    challenge: String,
}

async fn webhook_challenge(req: Query<ChallengeBody>) -> impl IntoResponse {
    (
        [
            ("Content-Type", "text/plain"),
            ("X-Content-Type-Options", "nosniff"),
        ],
        req.challenge.clone(),
    )
}

#[derive(Deserialize)]
struct Accounts {
    accounts: Vec<String>,
}

#[derive(Deserialize)]
struct Notification {
    list_folder: Accounts,
}

async fn capture_event(
    Json(req): Json<Notification>,
    Extension(db): Extension<Collection<UserData>>,
) -> impl IntoResponse {
    capture_event_inner(req.list_folder.accounts, db)
        .await
        .unwrap_or_else(|e| println!("capture_event error: {}", e));

    StatusCode::OK
}

#[derive(Deserialize)]
struct FileMetadata {
    #[serde(rename = ".tag")]
    tag: String,
    path_lower: String,
}

#[derive(Deserialize)]
struct Folders {
    cursor: String,
    entries: Vec<FileMetadata>,
    has_more: bool
}

async fn get_folders(access_token: &String, cursor: &mut String) -> Result<Vec<FileMetadata>, String> {
    let mut entries = Vec::new();

    loop {
        let mut folders = HTTP_CLIENT
        .post("https://api.dropboxapi.com/2/files/list_folder/continue")
        .bearer_auth(&access_token)
        .json(&json!({
            "cursor": cursor,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?

        .json::<Folders>()
        .await
        .map_err(|e| e.to_string())?;

        entries.append(&mut folders.entries);
        *cursor = folders.cursor;

        if !folders.has_more {
            break Ok(entries);
        }
    }
}

#[derive(Deserialize)]
struct Cursor {
    cursor: String,
}

async fn get_latest_cursor(access_token: String) -> Result<String, String> {
    HTTP_CLIENT
        .post("https://api.dropboxapi.com/2/files/list_folder/get_latest_cursor")
        .bearer_auth(access_token)
        .json(&json!({
            "include_deleted": false,
            "include_has_explicit_shared_members": true,
            "include_mounted_folders": true,
            "include_non_downloadable_files": false,
            "path": "",
            "recursive": true
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?

        .json::<Cursor>()
        .await
        .map(|c| c.cursor)
        .map_err(|e| e.to_string())
}

#[derive(Deserialize)]
struct SharedLink {
    url: String,
}

async fn create_shared_link(access_token: &String, path: &String) -> Result<String, String> {
    HTTP_CLIENT
        .post("https://api.dropboxapi.com/2/sharing/create_shared_link_with_settings")
        .bearer_auth(&access_token)
        .json(&json!({
            "path": path,
            "settings": {
                "access": "viewer",
                "allow_download": true,
                "audience": "public"
            }
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?

        .json::<SharedLink>()
        .await
        .map(|s| s.url)
        .map_err(|e| e.to_string())
}

async fn get_access_token_from_haiku(author_id: &String) -> Result<String, String> {
    HTTP_CLIENT
        .post(format!("{}/api/_funcs/_author_state", &*HAIKU_API_PREFIX))
        .header(header::AUTHORIZATION, HAIKU_AUTH_TOKEN)
        .json(&json!({ "author": author_id }))
        .send()
        .await
        .map_err(|e| e.to_string())?

        .text()
        .await
        .map(|at| decrypt(at))
        .map_err(|e| e.to_string())
}

async fn post_event_to_haiku(
    user: &String,
    text: &String,
    triggers: HashMap<String, String>
) -> Result<(), String> {
    HTTP_CLIENT
        .post(format!("{}/api/_funcs/_post", &*HAIKU_API_PREFIX))
        .header(header::AUTHORIZATION, &*HAIKU_AUTH_TOKEN)
        .json(&json!({
            "user": user,
            "text": text,
            "triggers": triggers,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())
        .and_then(|r| r.status().is_success().then_some(())
            .ok_or(format!("Failed to post event to haiku: {:?}", r)))
}

async fn capture_event_inner(
    accounts: Vec<String>,
    db: Collection<UserData>,
) -> Result<(), String> {
    for account in accounts {
        if let Some(mut userdata) = db
            .find_one(doc!{ "account_id": &account }, None)
            .await
            .map_err(|e| e.to_string())?
        {
            let access_token = get_access_token_from_haiku(&account).await?;
            let entries = get_folders(
                &access_token,
                &mut userdata.cursor
            )
            .await?;

            db.update_one(
                doc!{ "account_id": &account },
                doc!{ "$set": { "cursor": &userdata.cursor } },
                None
            )
            .await
            .map_err(|e| e.to_string())?;

            for entry in entries {
                if !entry.tag.eq("file") {
                    continue;
                }

                post_event_to_haiku(
                    &account,
                    &create_shared_link(&access_token, &entry.path_lower)
                        .await
                        .map_err(|e| format!("create_shared_link: {}", e))?,
                    [("event".to_string(), entry.tag)]
                        .into_iter()
                        .collect()
                )
                .await?;
            }
        } else {
            println!("Unregistered account: {}", account);
        }
    }

    Ok(())
}

async fn actions() -> impl IntoResponse {
    let actions = serde_json::json!({
        "list": [
            {
                "field": "To upload a file",
                "value": "upload_file",
                "desc": "This connector takes the return value of the flow function, and uploads it to the connected Dropbox API. It corresponds to the upload event in Dropbox API."
            }
        ]
    });
    Json(actions)
}

#[derive(Deserialize)]
struct HaikuRequest {
    user: String,
    state: String,
}

async fn events(
    req: Json<HaikuRequest>,
    Extension(db): Extension<Collection<UserData>>,
) -> impl IntoResponse {
    let cursor = match get_latest_cursor(decrypt(&req.state)).await {
        Ok(c) => c,
        Err(e) => return Err((StatusCode::INTERNAL_SERVER_ERROR,
            format!("get_latest_cursor: {}", e))),
    };

    db.insert_one(
        UserData {
            account_id: req.user.clone(),
            cursor
        },
        None
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let events = serde_json::json!({
        "list": [
            {
                "field": "Received a file",
                "value": "file",
                "desc": "This connector is triggered when a new file is uploaded to the connected Dropbox. It corresponds to the upload event in Dropbox API."
            }
        ]
    });
    Ok(Json(events))
}

#[derive(Serialize, Deserialize)]
struct UserData {
    account_id: String,
    cursor: String,
}

#[shuttle_service::main]
async fn axum(#[shared::MongoDb] db: Database) -> shuttle_service::ShuttleAxum {
    let db = db.collection::<UserData>("user_data");

    let router = Router::new()
        .route("/connect", get(connect))
        .route("/auth", get(auth))
        .route("/refresh", post(refresh))
        .route("/post", put(upload))
        .route("/actions", post(actions))
        .route("/events", post(events))
        .route("/webhook", get(webhook_challenge).post(capture_event))
        .layer(Extension(db));

    Ok(SyncWrapper::new(router))
}
