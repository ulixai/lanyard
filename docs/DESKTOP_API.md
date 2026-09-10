# Lanyard desktop protocol v2

Desktop applications and CLI tools can use Lanyard directly over JSON/HTTP. Python is optional. No internet service is involved. Ordinary web pages and browser extensions are deliberately unsupported.

Read `~/.lanyard/endpoint.json` before every connection. It contains `protocol: 2`, a numeric `port`, and an `instance` identifier. Connect only to `http://127.0.0.1:<port>`; disable proxies and HTTP redirects. The app binds to loopback only. Discovery files contain no credentials.

Create a random UUID `client_id` and a 32-byte cryptographically random `token`, encoded as unpadded base64url (43 characters). Save both in the operating system's credential store, scoped to your application. Reuse this pair across requests and launches. Never use the display name as authentication. A temporary identity works but requires pairing again next time.

POST `/v1/request`, with `Content-Type: application/json` and `X-Lanyard-Protocol: 2`:

```json
{
  "client_id": "a-random-UUID-generated-by-your-app",
  "token": "a-random-32-byte-base64url-secret",
  "app_name": "Your application",
  "reason": "Select a key for this connection.",
  "category": "api_key",
  "timeout": 300
}
```

Omit `target_id` to ask the user to choose a credential. Store the returned `target_id` in normal application settings; send it on future requests. An explicit target and optional category must match. Categories are `api_key`, `password`, `license`, `recovery`, `env`, and `crypto`. Reasons are limited to 2048 UTF-8 bytes; bodies to 16 KiB. Timeouts are integer seconds from 1 through 300; allow an additional five seconds on the client HTTP timeout. Consent shares all fields of one item. The desktop user chooses once, always, or deny. Automatic access requires an unlocked vault and a saved grant bound to the pairing token. Closing the HTTP connection cancels a pending request when the server observes it.

Successful response:

```json
{"status":"success","target_id":"credential-uuid","data":{"API_KEY":"secret"}}
```

Validate the response shape and target. Cap response reads at 2 MiB. Never log response bodies, headers containing identities, or tokens. Keep received secrets in memory only as long as necessary.

| HTTP status | Meaning |
| --- | --- |
| 400 | Invalid request or protocol headers |
| 403 | Denied, invalid pairing, or unsupported browser request |
| 404 | Target no longer exists |
| 408 | Consent expired or was cancelled |
| 426 | Legacy protocol; upgrade the app and SDK |
| 429 | Pending request queue is full; retry later |
| 503 | App temporarily unavailable |

GET `/v1/health` returns protocol, version, and instance. No endpoint exposes the vault inventory to external applications. Browser `Origin`/fetch metadata headers and a mismatched Host header are rejected; the API supplies no CORS permission.

## Trust boundary

Pairing proves possession of a saved token, not the executable publisher. Display names and reasons are supplied by the requesting application. Users must initiate pairing intentionally. The OS account/keychain remains the security boundary: malicious software with access to the user's process memory or unlocked keychain is outside the vault's isolation guarantees. Do not run Lanyard as an administrator or share its credential-store account across users.

## SDK 0.1 migration

Name-only permissions are intentionally not migrated. Upgrade the desktop and `lanyard` package to 0.2 together. The public Python `request_link`, `get_secret`, and `get_field` methods remain available. Legacy `/lanyard-ipc` responds with HTTP 426. A forgotten pairing can be recreated after the user forgets the app in Settings; it cannot inherit a previous app's permissions by copying its name.
