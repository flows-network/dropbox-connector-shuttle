# WasmHaiku API Reference

## WasmHaiku Request Parameters

| Name | Type | Required | Description |
| ---- | ---- | -------- | ----------- |
| user | string | &check; | The unique user identity in WasmHaiku. |
| state | string | &check; | The encrypted authorized access token. |
| refresh_state | string | &cross; | The encrypted authorized refresh token (Appears when [/refresh](./AUTHORIZE.md#refresh) is called). |
| text | string | &cross; | The string returned by the flow function (Appears when [/post](./OUTBOUND.md#post) is called with the `POST` method). |
| file | binary | &cross; | The file returned by the flow function (Appears when [/post](./OUTBOUND.md#post) is called with the `PUT` method). |
| forwards | [JSON](#forward-route-items) | &cross; | Multiple routes with user-selected items. |

### Routes

#### Forward route items

Multiple routes with user-selected items.

```json
{
    "<first-route>":
    [
        {
            "field": "Item-1",  // The name of the route item displayed
            "value": "value-1"  // The value of the route item
        },
        {
            "field": "Item-2",  // The name of the route item displayed
            "value": "value-2"  // The value of the route item
        }
        // ...
    ],
    "<second-route>": [ /* ... */ ]
}
```

#### Route item list

A list of items in a route.

```json
{
    "next_cursor": "xxx",   // If there is no more data it should be omitted.
    "list": 
    [
        {
            "field": "Item-1",  // The name of the route item displayed
            "value": "value_1", // The value of the route item
            "desc": "Route item 1 description"
        }
        {
            "field": "Item-2",  // The name of the route item displayed
            "value": "value_2", // The value of the route item
            "desc": "Route item 2 description"
        }
        // ...
    ]
}
```
