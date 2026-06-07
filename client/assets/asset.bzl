load("@rules_rust//rust:defs.bzl", "rust_library")

def embed_asset(name, src, variable_name):
    rs_out = name + ".rs"
    native.genrule(
        name = name + "_gen",
        srcs = [src],
        outs = [rs_out],
        tools = ["//client/winit/tools:embed_rs"],
        cmd = "$(location //client/winit/tools:embed_rs) $(location %s) $(location %s) %s" % (src, rs_out, variable_name),
    )

    rust_library(
        name = name,
        srcs = [rs_out],
        visibility = ["//visibility:public"],
    )

def asset_registry(name, assets):
    rs_out = name + ".rs"
    
    args = []
    for a in assets:
        args.append("'%s'" % a["key"])
        args.append("'%s'" % a["crate"])
        args.append("'%s'" % a["var"])
    
    native.genrule(
        name = name + "_gen",
        srcs = ["//client/assets:tools/gen_registry.py"],
        outs = [rs_out],
        cmd = "python3 $(location //client/assets:tools/gen_registry.py) $(location %s) %s" % (rs_out, " ".join(args)),
    )
    
    rust_library(
        name = name,
        srcs = [rs_out],
        deps = [a["dep"] for a in assets],
        visibility = ["//visibility:public"],
    )
