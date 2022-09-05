# Authorize

Nowadays, the application platform often uses [OAuth 2.0](https://oauth.net/) to authorize access to a user’s data. You can also refer to the [Dropbox OAuth Guide](https://www.dropbox.com/lp/developers/reference/oauth-guide).

Here we use [code flow](https://oauth.net/2/grant-types/authorization-code/) to obtain the user authorization, the step are as follows.

* When the user clicks `+ Authenticate Account`, WasmHaiku calls [/connect](#connect), and then [/connect](#connect) constructs a Dropbox authorization URL with your application's `client_id` and `redirect_uri`, and redirects to the Dropbox authorization page.
* Wait for the end user to complete authorization on Dropbox authorization page, whom is then redirected back to [/auth](#auth) with an authorization code in the query string.
* [/auth](#auth) calls Dropbox's [/oauth/token][o2t] to get the `access_token` and `refresh_token`, and redirects the encryped tokens and other informations back to WasmHaiku.

## /connect

`/connect` returns a `302 Found` redirect response with a Dropbox authorization URL:

<https://www.dropbox.com/oauth2/authorize?>

* `client_id=<DROPBOX_APP_CLIENT_ID>&`
* `redirect_uri=<SERVICE_API_PREFIX>auth&` Redirected back to [/auth](#auth) to get the tokens.
* `response_type=code&` Verify with code flow.
* `token_access_type=offline` Makes [/oauth2/token][o2t] returns  a short-lived __access_token__ and a long-lived __refresh_token__ that can be used to request a new short-lived access token as long as a user's approval remains valid.

```rust
return (StatusCode::FOUND, [(header::LOCATION, format!(
        "https://www.dropbox.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&token_access_type=offline",
        &*DROPBOX_APP_CLIENT_ID,
        urlencoding::encode(&*REDIRECT_URL)
    )
)]);
```

## /auth

When the user complete authentication, the [/auth](#auth) will be called with the authorization code. The code to call [/oauth2/token][o2t] as follows (The implementation is simplified here for ease of understanding):

```rust
#[derive(Deserialize, Clone)]
struct AccessToken {
    access_token: String,
    refresh_token: String,
    account_id: Option<String>,
}
// snip
let at = HTTP_CLIENT         // reqwest Client
    .post("https://api.dropbox.com/oauth2/token")

    // Basic auth with your APP_ID and APP_SECRET
    .basic_auth(DROPBOX_APP_CLIENT_ID, Some(DROPBOX_APP_CLIENT_SECRET))

    // Send a form body
    .form(&[
        ("code", code),
        ("grant_type", "authorization_code".to_string()),
        ("redirect_uri", format!("{}/auth", SERVICE_API_PREFIX))
    ])
    .send()
    .await
    .unwrap()

    // Deserialize JSON body to AccessToken
    .json::<AccessToken>
    .unwrap();
```

After we get the tokens, we returns a redirect response back to the WasmHaiku (Here we omit the step of getting the account name).

```rust
return (StatusCode::FOUND, [(header::LOCATION, format!(
        "{}/api/connected?authorId={}&authorName={}&authorState={}&refreshState={}",
        HAIKU_API_PREFIX,
        id,
        format!("{} ({})", account.name.display_name, account.email),
        encrypt(&at.access_token),
        encrypt(&at.refresh_token)
    ))]);
```

Congratulations, user authentication is now complete!

## /refresh

TODO

[o2t]: https://www.dropbox.com/developers/documentation/http/documentation#oauth2-token
