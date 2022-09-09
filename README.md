# How to Build a WasmHaiku Connector with shuttle.rs

[WasmHaiku][w] Connector is the bridge between applications and [WasmHaiku][w]. It receives data from an application then sends it to [WasmHaiku][w] and vice versa.

But in case you can't find a connector implemented already for the applications you're interested in, or certain applications don't provide public APIs, you will need to build a [WasmHaiku][w] Connector yourself.

[shuttle.rs][sh] provides a platform for building web applications. In this tutorial we will show you how to program with [shuttle.rs][sh] and deploy your application to [shuttle.rs][sh]. The case study here is [Dropbox][d], you can also see the [Dropbox API Reference](https://www.dropbox.com/developers/documentation/http/documentation).

## Before you start

For the purpose of this tutorial, you need to do the following:

* Confirm that you have an account with [WasmHaiku][w], [Dropbox][d], [Slack][sl] and [shuttle.rs](https://www.shuttle.rs/login).
* Install [shuttle.rs][sh] with `cargo`.

  ```shell
  cargo install cargo-shuttle
  ```

* Login to [shuttle.rs][sh] with your API key.
  
  ```shell
  cargo shuttle login --api-key <YOUR API KEY>
  ```

* Create your project with [axum](https://docs.rs/axum/latest/axum/) as the server framework.
  
  ```shell
  cargo shuttle init --axum <YOUR PROJECT NAME>
                          # dropbox-connector-shuttle
  ```

## Build your WasmHaiku Connector

As a service, the Connector should reside to keep listening to the requests coming from the application and [WasmHaiku][w]. It should provide APIs that handle these things:

* [Authorize](./AUTHORIZE.md) - Recognize the user of the application and send the user's authorized state to [WasmHaiku][w].
* [Inbound](./INBOUND.md) - Listen to the event popped up by the application and send it to [WasmHaiku][w].
* [Outbound](./OUTBOUND.md) - Receive the data from [WasmHaiku][w] and send it to the application.
* [Deploy](./DEPLOY.md)

Every Connector should implement the Authorize API, and one or both of the Inbound and the Outbound API.

You can find the complete code for this tutorial [here](https://github.com/second-state/dropbox-connector-shuttle). If you have any questions or suggestions, please open an issue or pull request for free.

[d]: https://www.dropbox.com
[w]: https://flows.network
[sh]: https://shuttle.rs
[sl]: https://slack.com
