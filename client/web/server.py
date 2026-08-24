import http.server
import socketserver
import os

PORT = 8080

def get_runfiles_dir():
    if "RUNFILES_DIR" in os.environ:
        return os.environ["RUNFILES_DIR"]
    if "PYTHON_RUNFILES" in os.environ:
        return os.environ["PYTHON_RUNFILES"]
    return os.getcwd()

RUNFILES_DIR = get_runfiles_dir()
ROUTE_MAP = {}

def init_routes():
    print(f"Scanning in: {RUNFILES_DIR}")
    for root, _, files in os.walk(RUNFILES_DIR):
        for f in files:
            full_path = os.path.join(root, f)

            # Map index and static assets
            if f == "index.html":
                ROUTE_MAP["/"] = full_path
                ROUTE_MAP["/index.html"] = full_path
                ROUTE_MAP["/static/index.html"] = full_path
            elif f == "style.css":
                ROUTE_MAP["/style.css"] = full_path
                ROUTE_MAP["/static/style.css"] = full_path

            # Map generated WASM and JS files
            if f.endswith((".js", ".wasm", ".d.ts")):
                ROUTE_MAP[f"/{f}"] = full_path
                ROUTE_MAP[f"/pkg/{f}"] = full_path

class WASMHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "application/javascript",
        ".css": "text/css",
        ".html": "text/html",
    }

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()

    def do_GET(self):
        clean_path = self.path.split("?")[0].split("#")[0]

        # Handle favicon requests gracefully
        if clean_path == "/favicon.ico":
            self.send_response(204)
            self.end_headers()
            return

        # Serve JS files from source
        if clean_path == "/src/main.js":
            js_path = os.path.join(RUNFILES_DIR, "client/web/src/main.js")
            if not os.path.isfile(js_path):
                js_path = os.path.join(os.getcwd(), "client/web/src/main.js")
            if serve_file(self, js_path, "text/javascript"):
                return

        if clean_path == "/src/worker.js":
            js_path = os.path.join(RUNFILES_DIR, "client/web/src/worker.js")
            if not os.path.isfile(js_path):
                js_path = os.path.join(os.getcwd(), "client/web/src/worker.js")
            if serve_file(self, js_path, "text/javascript"):
                return

        # Serve route-mapped files
        if clean_path in ROUTE_MAP:
            target_file = ROUTE_MAP[clean_path]
            if os.path.isfile(target_file):
                ext = os.path.splitext(target_file)[1]
                content_type = self.extensions_map.get(ext, "application/octet-stream")
                serve_file(self, target_file, content_type)
                return

        super().do_GET()

def serve_file(handler, path, content_type):
    if os.path.isfile(path):
        handler.send_response(200)
        handler.send_header("Content-Type", content_type)
        handler.send_header("Content-Length", str(os.path.getsize(path)))
        handler.end_headers()
        with open(path, "rb") as f:
            handler.wfile.write(f.read())
        return True
    return False

if __name__ == "__main__":
    init_routes()

    print("\n--- Registered Active Routes ---")
    for path, target in sorted(ROUTE_MAP.items()):
        print(f"  http://localhost:{PORT}{path} -> {target}")
    print("--------------------------------\n")

    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.TCPServer(("", PORT), WASMHandler) as httpd:
        print(f"Server running at http://localhost:{PORT}")
        print("WARNING: SharedArrayBuffer requires HTTPS or localhost")
        print(f"  Headers sent: COOP=same-origin, COEP=require-corp")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nServer stopped.")
