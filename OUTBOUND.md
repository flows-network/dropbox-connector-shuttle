# Outbound

Acting as outbound, the Connector provides an API, which is usually  [/post](#post), for transferring flow data coming 
from 
WasmHaiku to the application.

After authorization, WasmHaiku will get the list of items in the routes, which is `action` route here, and WasmHaiku 
will call [/actions](#actions) to be used for the outbound, which specifies the outbound parameters (eg. outbound 
target 
channel, outbound message type etc.).

When the flow function executes successfully, WasmHaiku will send a request to [/post](#post), and then [/post](#post) transform the data and send it to the application.

## /actions

Dropbox outbound has only one route `action` and it has only one item `To upload a file`. When the user sets up the outbound and selects the `action`, WasmHaiku uses the `POST` method to send a request to [/actions](#actions) with [WasmHaiku request parameters](./API-REFERENCE.md#wasmhaiku-request-parameters) in JSON format, but here we omit these parameters.

After receiving the request, we need to return a [list](./API-REFERENCE.md#route-item-list) of `action`'s routing 
items. Axum implementation is as follows:

```rust
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
```

## /post

Dropbox uses raw data to upload files, so here WasmHaiku uses the `PUT` method to send a request to [/post](#post) with [WasmHaiku request parameters](./API-REFERENCE.md#wasmhaiku-request-parameters) in multipart form, and we receive the file first.

```rust
async fn upload(mut multipart: Multipart)
// snip
let mut access_token = None; 
let mut file = Vec::new();
let mut file_name = None;

while let Some(field) = multipart.next_field().await.unwrap_or_else(|_| None) {
    match field.name().unwrap_or_default() {
        "file" => {
            file.append(&mut field.bytes().await.unwrap().into());
        },
        "text" => {
            file_name = Some(String::from_utf8(
                field.bytes().await.unwrap().to_vec()).unwrap());
        }
        "state" => {
            access_token = Some(decrypt(field.bytes().await.unwrap()));
        }
         _ => {},
    }
}
```

After receiving the file, we need to upload the file via [/upload](https://www.dropbox.com/developers/documentation/http/documentation#files-upload), let's see the implementation:

```rust
HTTP_CLIENT
    .post("https://content.dropboxapi.com/2/files/upload")
    .bearer_auth(access_token)
    .header(header::CONTENT_TYPE, "application/octet-stream")
    .header("Dropbox-API-Arg",
        json!({
            "autorename": true, // Dropbox server try to autorename the file if there's conflict
            "path": file_name,  // Path in the user's Dropbox to save the file.
        }).to_string())
    .body(file)
    .send()
    .await
```

At this point, the implementation of the outbound has been completed.
