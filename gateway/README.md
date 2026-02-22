# Gateway

API gateway for the Nexus IoT platform. Handles user authentication (OAuth 2.0 / Google) and provides the REST API for
device management.

## Dependencies

- PostgreSQL 16
- Rust 1.91+

### Config file

Built-in defaults are used when no file is provided. Pass `--config config.json` to override:

```json
{
  "api": {
    "listen_addr": "0.0.0.0:8000",
    "oauth": {
      "base_url": "https://api.example.com",
      "jwt": {
        "expires_in": 86400
      }
    }
  },
  "postgres": {
    "db_pool_size": 5
  }
}
```
