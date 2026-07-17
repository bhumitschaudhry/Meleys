You are implementing Task 14 of a multi-engine architecture for Meleys (a Rust browser runtime).

**Project context:** Meleys is at E:\Meleys. Tasks 1-13 completed the engine abstraction, config, fallback, session management, and removed capture actions. Now we update CI/CD.

**Task 14: CI/CD Pipeline Updates**

Modify `.github/workflows/ci.yml`:

1. Add a Lightpanda binary download step for Linux in the test job (before the test steps):

```yaml
      - name: Install Lightpanda (Linux only)
        if: runner.os == 'Linux'
        run: |
          curl -sL https://github.com/lightpanda-io/browser/releases/latest/download/lightpanda-x86_64-linux -o /usr/local/bin/lightpanda
          chmod +x /usr/local/bin/lightpanda
          lightpanda --version || echo "Lightpanda installed"
```

2. Add an env var to the test job:
```yaml
    env:
      MELEYS_ENGINE_DEFAULT: chromium
```

This ensures CI tests use Chromium (since Lightpanda may not be available on all runners).

3. Commit: `ci: add Lightpanda setup and engine configuration`

Return: status, commits, test results, concerns.
