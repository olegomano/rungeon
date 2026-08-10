def _wasm_transition_impl(settings, attr):
    return {"//command_line_option:platforms": "@rules_rust//rust/platform:wasm"}

wasm_transition = transition(
    implementation = _wasm_transition_impl,
    inputs = [],
    outputs = ["//command_line_option:platforms"],
)

def _wasm_target_impl(ctx):
    # ctx.files.src automatically extracts all output files from the src target
    return [DefaultInfo(files = depset(ctx.files.src))]

wasm_target = rule(
    implementation = _wasm_target_impl,
    attrs = {
        "src": attr.label(cfg = wasm_transition, mandatory = True),
        "_allowlist_function_transition": attr.label(
            default = "@bazel_tools//tools/allowlists/function_transition_allowlist",
        ),
    },
)
