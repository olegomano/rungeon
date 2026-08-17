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
    print(f"Scanning runfiles in: {RUNFILES_DIR}")
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

                # Set up aliases if index.html requests /pkg/web_lib.* instead of /pkg/web_wasm.*
                if f.endswith(".js"):
                    ROUTE_MAP["/pkg/web_lib.js"] = full_path
                elif f.endswith("_bg.wasm"):
                    ROUTE_MAP["/pkg/web_lib.wasm"] = full_path

class WASMHandler(http.server.SimpleHTTPRequestHandler):
    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "application/javascript",
        ".css": "text/css",
        ".html": "text/html",
    }

    def do_GET(self):
        clean_path = self.path.split("?")[0].split("#")[0]

        if clean_path in ROUTE_MAP:
            target_file = ROUTE_MAP[clean_path]
            if os.path.isfile(target_file):
                self.send_response(200)
                ext = os.path.splitext(target_file)[1]
                content_type = self.extensions_map.get(ext, "application/octet-stream")
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(os.path.getsize(target_file)))
                self.end_headers()

                with open(target_file, "rb") as f:
                    self.wfile.write(f.read())
                return

        super().do_GET()

if __name__ == "__main__":
    init_routes()

    print("\n--- Registered Active Routes ---")
    for path, target in sorted(ROUTE_MAP.items()):
        print(f"  http://localhost:{PORT}{path} -> {target}")
    print("--------------------------------\n")

    socketserver.TCPServer.allow_reuse_address = True
    with socketserver.TCPServer(("", PORT), WASMHandler) as httpd:
        print(f"Server running at http://localhost:{PORT}")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nServer stopped.")
