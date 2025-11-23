# Security Policy

## Supported Versions

SecBeat is in early development (v0.1.x as of November 2025). Security updates are provided on a best-effort basis.

| Version | Supported          | Status      |
| ------- | ------------------ | ----------- |
| 0.1.x   | :white_check_mark: | Development |
| < 0.1   | :x:                | Deprecated  |

**Note**: This project is not recommended for production use. No formal security SLA is provided.

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please follow responsible disclosure:

### How to Report

**DO NOT** open a public GitHub issue for security vulnerabilities.

Instead, report via:

1. **Email**: Send to the repository maintainer or use GitHub Security Advisories
2. **Subject Line**: "SecBeat Security Vulnerability Report"
3. **Include**:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)
   - Your contact information for follow-up

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 1 week
- **Status Updates**: Every 2 weeks until resolved
- **Disclosure Timeline**: Coordinated disclosure after patch release

### Response Process

1. **Triage**: We assess severity and impact
2. **Investigation**: Reproduce and analyze the issue
3. **Fix Development**: Create and test patch
4. **Disclosure**: Coordinate public disclosure with reporter
5. **Credit**: Public acknowledgment of reporter (with permission)

## Security Best Practices

If deploying SecBeat (not recommended for production), follow these guidelines:

### Network Security

- Run behind a dedicated firewall
- Isolate from internal networks
- Use TLS 1.3 with strong cipher suites
- Implement rate limiting at network edge

### System Security

- Run with minimal required privileges
- Use Linux capabilities instead of root when possible
- Keep system packages updated
- Monitor logs for suspicious activity

### Configuration Security

- Change default API keys immediately
- Use strong, random secrets
- Rotate credentials regularly
- Restrict API access by IP when possible

### Docker Security

- Use official base images only
- Scan images for vulnerabilities
- Run containers with read-only root filesystem
- Limit container capabilities

## Known Security Limitations

### Current Development Status

- **Early Stage**: Not audited for production use
- **Experimental Features**: SYN proxy and eBPF code are prototypes
- **Limited Testing**: Security testing is incomplete
- **No Guarantees**: No warranty or liability for security issues

### Specific Concerns

1. **SYN Proxy**: Experimental implementation, potential bypass vectors
2. **eBPF/XDP**: Requires CAP_NET_RAW, potential privilege escalation
3. **WASM Runtime**: Sandbox limitations not fully validated
4. **Input Validation**: WAF rules may have bypass vulnerabilities
5. **Dependency Security**: Third-party crates not independently audited

## Security Roadmap

Planned security improvements:

- [ ] Professional security audit
- [ ] Fuzzing integration (cargo-fuzz)
- [ ] Dependency vulnerability scanning (cargo-audit)
- [ ] Static analysis (cargo-clippy security lints)
- [ ] Penetration testing
- [ ] Secure defaults enforcement
- [ ] Security documentation
- [ ] Threat model documentation

## Disclosure Policy

### Coordinated Disclosure

- **Embargo Period**: 90 days from initial report
- **Public Disclosure**: After patch release or embargo expiration
- **CVE Assignment**: Requested for high/critical issues
- **Security Advisories**: Published on GitHub Security Advisories

### Severity Classification

We use CVSS 3.1 for severity scoring:

- **Critical** (9.0-10.0): Immediate action required
- **High** (7.0-8.9): Urgent fix needed
- **Medium** (4.0-6.9): Important fix
- **Low** (0.1-3.9): Minor issue

## Legal

### Safe Harbor

We support security research conducted in good faith. Researchers who:

- Report vulnerabilities responsibly
- Avoid compromising user data
- Do not exploit vulnerabilities beyond proof-of-concept
- Follow this disclosure policy

Will not face legal action from the project maintainers.

### Out of Scope

The following are explicitly out of scope:

- Denial of Service attacks on live infrastructure
- Social engineering of maintainers or users
- Physical security testing
- Third-party services or dependencies
- Issues in outdated/unsupported versions

## Contact

- **Security Email**: security@example.com (replace with actual contact)
- **PGP Key**: Available at https://example.com/pgp-key.txt (if applicable)
- **GitHub**: https://github.com/fabriziosalmi/secbeat/security/advisories

## Attribution

Thank you to the security researchers who have responsibly disclosed vulnerabilities:

- (None yet - project is in early development)

---

**Last Updated**: 2025-11-24  
**Version**: 1.0
