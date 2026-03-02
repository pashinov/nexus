# Certificates

## Generate

```bash
./generate-ca.sh
```

By default the server certificate is issued for `mqtt.apashinov.com`.
Override with `SERVER_DOMAIN`:

```bash
SERVER_DOMAIN=mqtt.example.com ./generate-ca.sh
```

## Output

| File | Description | Deploy to |
|------|-------------|-----------|
| `ca.pem` | CA certificate | EMQX + devices |
| `server.pem` | Server certificate | EMQX |
| `server.key` | Server private key | EMQX (keep secret) |
| `client.pem` | Client certificate | Devices |
| `client.key` | Client private key | Devices (keep secret) |

## Upload to EMQX

1. Open EMQX Dashboard → **Management → Listeners → ssl:default → Edit**
2. Upload `ca.pem`, `server.pem`, `server.key`
3. Enable **Verify Peer** and **Force Verify Peer Certificate**
4. Disable **Enable Authentication**
5. Save

## Deploy to device

Copy to device:
```
/etc/mqtt-client/certs/ca.pem
/etc/mqtt-client/certs/client.pem
/etc/mqtt-client/certs/client.key
```
