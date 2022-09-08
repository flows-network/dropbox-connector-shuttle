# Inbound

Acting as inbound, the Connector should be responsible for transferring event data coming from the application to WasmHaiku. It needs to provide an API, which usually is [/webhook](#webhook), for the application to report events. You can also refer to the [Dropbox Webhooks Tutorial](https://www.dropbox.com/developers/reference/webhooks).

After authorization, WasmHaiku will get the list of items in the `trigger routes`, which is `event` here, and WasmHaiku will call [/events](#events) to be used for the inbound, which specifies the events that WasmHaiku triggers the `flow function` (eg. interested channel etc.).

Before Dropbox begin to report events by sending HTTP requsets to [/webhook](#events-capture-post) using the `POST` method, it needs to confirm that Dropbox is communicating with the right service using a [verification request](#verification-request-get) with the `GET` method.

## /events

Because Dropbox requires a cursor to access the changes, we not only need to return a list of `trigger route` items, but also need to get the user's latest cursor and store it in to the database provided by shuttle.rs.

```rust
async fn events(
    // state: String // access_token,
    // user: String  // account_id
    req: Json<HaikuRequest>,
    Extension(db): Extension<Collection<UserData>>,
)
// snip
let cursor = get_latest_cursor(decrypt(&req.state))
    .await.unwrap();

db.insert_one(UserData
    { account_id: req.user.clone(), cursor }, None)
    .await.unwrap();

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
```

## /webhook

### Verification Request (GET)

This verification is an HTTP `GET` request with a query parameter called `challenge`. Your app needs to respond by echoing back that `challenge` parameter. Axum implementation is as follows:

```rust
async fn webhook_challenge(
    // challenge: String
    req: Query<ChallengeBody>
) -> impl IntoResponse {
    (
        [
            // In order to avoid introducing a reflected XSS vulnerability
            ("Content-Type", "text/plain"),
            ("X-Content-Type-Options", "nosniff"),
        ],
        req.challenge.clone(),
    )
}
```

### Events Capture (POST)

When there's a change in any of the account connected to your connector, Dropbox uses the `POST` method to send a request to [/webhook](#webhook) with [notification body](https://www.dropbox.com/developers/reference/webhooks#documentation) which including changed accounts.

After receiving the notification, we need to query the cursor in database with the `account_id`, then use the cursor to access the changes and finally post the event to WasmHaiku.

```rust
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
)
// snip
for account in req.list_folder.accounts {
    if let Some(mut userdata) = db
        .find_one(doc!{ "account_id": &account }, None)
        .await.unwrap()
    {
        let access_token = get_access_token_from_haiku(&account).await?;

        // Use cursor to access changes
        let entries = get_folders(
            &access_token,
            &mut userdata.cursor
        )
        .await?;

        // Update the latest cursor
        db.update_one(
            doc!{ "account_id": &account },
            doc!{ "$set": { "cursor": &userdata.cursor } },
            None
        )
        .await;

        for entry in entries {
            if !entry.tag.eq("file") {
                continue;   // Ignore other events
            }

            post_event_to_haiku(
                &account,

                // Convert paths to links as inbound data
                &create_shared_link(&access_token, &entry.path_lower)
                    .await?,

                // Set up trigger
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
```
