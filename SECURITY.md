# Security Policy

## Scope

chromaport reads VS Code/Cursor theme files and writes configuration files
to application-specific directories. Security issues in scope include:

- Path traversal (writing files outside intended directories)
- Arbitrary file overwrite through crafted theme data
- Command injection through theme metadata

Out of scope:

- Issues in upstream editors (VS Code, Cursor) or target applications
- Expected CLI behavior and output

## Reporting a Vulnerability

Please report security vulnerabilities by emailing **zlemzlem5656@naver.com**.

**Do NOT open a public GitHub issue for security vulnerabilities.**

We aim to acknowledge reports within 7 days and provide a fix or
mitigation plan within 30 days.

## Supported Versions

Only the latest release is supported with security updates.
