# WASM Filter

A WebAssembly-based content filtering proxy for Bord, a social media platform. Provides filtering and sanitization of user-generated content.

## Setup

The main Bord application should be spun to a different port (e.g., `--port 3001`) to allow the filter proxy to run on the default port and forward requests appropriately.

```bash
cd ../
spin up --listen 127.0.0.1:3001
```
