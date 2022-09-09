# Deploy

Once the connector is complete, we need it to run and work with WasmHaiku and connect applications.

- [Deploy](#deploy)
  - [Deploy your connector to shuttle.rs](#deploy-your-connector-to-shuttlers)
  - [Register the connector on WasmHaiku](#register-the-connector-on-wasmhaiku)
  - [Register the connector on Dropbox](#register-the-connector-on-dropbox)

## Deploy your connector to shuttle.rs

First of all, we need to get our service running and publicly accessible over the Internet. Of cause you can run your connector locally and use `Intranet penetration` services like `ngrok`, but here we using a cloud-native service provided by shuttle.rs.

Note that you need to commit your changes to git repository before deploying.

```shell
git commit -a -m "Initial commit"
cargo shuttle deploy
```

or

```shell
cargo shuttle deploy --allow-dirty
```

After deployment, you can do some testing to make sure that your connector is accessible (eg. call `https://<PROJECT_NAME>.shuttleapp.rs/connect`).

## Register the connector on WasmHaiku

Log in to WamsHaiku and go to [My Connectors](https://flows.network/connector) page, and click `Create Connector` to create a new connector.

Feel free type `Name`, `Desc`, `Webhook Prompt`, `Repo` and `README.md`, then we'll talk about the remaining options.

- __Auth Token__ - Fill it to the `HAIKU_AUTH_TOKEN` of your connector.
- __Authorization URL__ - It should be `https://<PROJECT_NAME>.shuttleapp.rs/connect`.
- __State Refresh URL__ - It should be `https://<PROJECT_NAME>.shuttleapp.rs/refresh`.
- __Trigger Route__:

  ```json
  {
      "routes": [
          {
              "route": "event",
              "title": "event",
              "list": " https://<PROJECT_NAME>.shuttleapp.rs/events",
              "multi": false
          }
      ]
  }
  ```

- __Forward URL__ - It should be `https://<PROJECT_NAME>.shuttleapp.rs/post`
- __Forward Route__:
  
  ```json
  {
      "routes": [
          {
              "route": "action",
              "title": "action",
              "list": " https://<PROJECT_NAME>.shuttleapp.rs/actions",
              "multi": false
          }
      ]
  }
  ```

Finally, click the `Save` button.

## Register the connector on Dropbox

Log in to Dropbox and go to the [App Console](https://www.dropbox.com/developers/apps), selects your application and focus on the `Settings` page, you will see the options described below.

- `App key` - Fill it to the `DROPBOX_APP_CLIENT_ID` of your connector.
- `App secret` - Fill it to the `DROPBOX_APP_CLIENT_SECRET` of your connector.
- `OAuth 2 -> Redirect URIs` - It should he `https://<PROJECT_NAME>.shuttleapp.rs/auth`
- `Webhooks -> Webhooks URIs` - It should be `https://<PROJECT_NAME>.shuttleapp.rs/webhook`.

After that, you should go to the `Permissons` page, then select the `files.content.write` and `sharing.write`, and finally click `Submit` at the bottom of the page.

Now, your connector is ready, you can use it as inbound or outbound for your flow function. Happy to use!
