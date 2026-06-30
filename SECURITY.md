# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 1.0.x   | Yes |
| < 1.0   | No |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in Atrium, please follow the responsible disclosure process below.

**Do NOT open a public GitHub issue for security vulnerabilities.**

### How to Report

1. Email the details to the maintainer at **chinoshizuyuki@gmail.com**.
2. Include a description of the vulnerability and steps to reproduce it.
3. If possible, include a proof-of-concept or suggested fix.

### What to Expect

- **Acknowledgment** — We will acknowledge receipt of your report within 48 hours.
- **Assessment** — We will assess the severity and impact within 7 days.
- **Fix** — We aim to release a fix for confirmed vulnerabilities within 30 days.
- **Disclosure** — We will coordinate the public disclosure with you after the fix is released.

### Scope

The following are in scope:

- The Rust core engine (`crates/`)
- Python gateway and services (`services/`)
- Docker Compose deployment (`docker-compose.yml`, `Dockerfile`)
- gRPC and HTTP API surfaces
- Configuration handling (`atrium.toml`, environment variables)

The following are out of scope:

- Third-party dependencies (report to their respective maintainers)
- Issues in unsupported versions
- Social engineering or phishing attacks

### Safe Harbor

We consider security research conducted in accordance with this policy to be authorized and will not pursue legal action against researchers who:

- Report vulnerabilities before disclosing them publicly
- Avoid privacy violations, data destruction, and service disruption
- Only interact with accounts they own or with explicit permission

## Security Best Practices for Users

When deploying Atrium, keep these in mind:

- **Change default passwords** — Docker Compose uses `atrium` as the default password for PostgreSQL, Redis, and Grafana. Set strong passwords via environment variables (`POSTGRES_PASSWORD`, `REDIS_PASSWORD`, `GRAFANA_PASSWORD`).
- **Restrict CORS origins** — Set `CORS_ORIGINS` to your actual frontend domains instead of allowing all origins.
- **Use HTTPS in production** — Place a reverse proxy (nginx, Caddy) with TLS in front of the gateway.
- **Keep dependencies updated** — Run `cargo update` and `pip install --upgrade` regularly.
- **Limit network exposure** — Bind the gateway to `localhost` or a private network unless you explicitly need external access.
