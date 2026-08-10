import http.server
import socketserver
import os

PORT = 8080

# Find the runfiles directory for Bazel
RUNFILES_DIR = os.environ.get("RUNFILES_DIR", os.path.dirname(__file__))

class Handler(http.server.SimpleHTTPRequestHandler):
    def translate_path(self, path):
        # Map /pkg/web_lib.js and /pkg/web_lib.wasm to the actual files
        # The wasm_target rule exposes the files from generate_bindings
        # which are typically named like: web_bindings.js, web_bindings_bg.wasm
        
        if path == '/pkg/web_lib.js':
            # Look for the .js file
            for root, dirs, files in os.walk(RUNFILES_DIR):
                for f in files:
                    if f.endswith('.js') and 'web_bindings' in f:
                        return os.path.join(root, f)
                    elif f.endswith('.js') and 'generate_bindings' in root:
                        return os.path.join(root, f)
        elif path == '/pkg/web_lib.wasm':
            # Look for the .wasm file
            for root, dirs, files in os.walk(RUNFILES_DIR):
                for f in files:
                    if f.endswith('.wasm') and ('web_bindings' in f or 'web_wasm' in f or 'generate_bindings' in root):
                        return os.path.join(root, f)
        
        # Default behavior
        return super().translate_path(path)

    def do_GET(self):
        if self.path.endswith('.wasm'):
            self.send_response(200)
            self.send_header('Content-Type', 'application/wasm')
            self.end_headers()
            try:
                path = self.translate_path(self.path)
                with open(path, 'rb') as f:
                    self.wfile.write(f.read())
            except (FileNotFoundError, OSError) as e:
                self.send_error(404, f"File not found: {self.path}")
        elif self.path.endswith('.js'):
            self.send_response(200)
            self.send_header('Content-Type', 'application/javascript')
            self.end_headers()
            try:
                path = self.translate_path(self.path)
                with open(path, 'rb') as f:
                    self.wfile.write(f.read())
            except (FileNotFoundError, OSError) as e:
                self.send_error(404, f"File not found: {self.path}")
        else:
            super().do_GET()

if __name__ == "__main__":
    print(f"Serving from {RUNFILES_DIR} at http://localhost:{PORT}")
    print("Press Ctrl+C to stop")
    with socketserver.TCPServer("", PORT, Handler) as httpd:
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nServer stopped")
