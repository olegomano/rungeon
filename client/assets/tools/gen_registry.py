import json
import sys

def main():
    out_file = sys.argv[1]
    # Assets are passed as triplets: key, crate, var
    args = sys.argv[2:]
    assets = []
    for i in range(0, len(args), 3):
        assets.append({
            "key": args[i],
            "crate": args[i+1],
            "var": args[i+2]
        })
    
    with open(out_file, "w") as f:
        for a in assets:
            f.write(f"extern crate {a['crate']};\n")
        f.write("\n")
        f.write("pub fn get_asset(name: &str) -> Option<&'static [u8]> {\n")
        f.write("    match name {\n")
        for a in assets:
            f.write(f"        \"{a['key']}\" => Some({a['crate']}::{a['var']}),\n")
        f.write("        _ => None,\n")
        f.write("    }\n")
        f.write("}\n")

if __name__ == "__main__":
    main()
