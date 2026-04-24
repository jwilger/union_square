---
name: security-review
description: Audit code for security vulnerabilities, unsafe practices, and data exposure risks
license: MIT
compatibility: opencode
---

## What I do
- Identify input validation gaps
- Check for hardcoded secrets or credentials
- Review error messages for information leakage
- Validate dependency security (cargo audit)
- Check for unsafe Rust usage

## When to use me
Use this skill when reviewing code that handles external input, authentication, authorization, or sensitive data.

## Review Checklist
- [ ] All external inputs validated
- [ ] No hardcoded secrets
- [ ] Error messages don't leak internals
- [ ] Database queries are parameterized
- [ ] No unnecessary unsafe blocks
- [ ] Rate limiting considered for APIs
