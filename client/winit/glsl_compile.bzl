def _glsl_to_rs_impl(ctx):
    input_file = ctx.file.src
    output_name = ctx.attr.out if ctx.attr.out else input_file.basename + ".rs"

    spirv_out = ctx.actions.declare_file(input_file.basename + ".spv")
    rust_out = ctx.actions.declare_file(output_name)

    # Compile GLSL → SPIR-V
    ctx.actions.run(
        inputs = [input_file],
        outputs = [spirv_out],
        arguments = [
            "-V",
            "-o", spirv_out.path,
            input_file.path,
        ],
        executable = "glslangValidator",
        progress_message = "Compiling GLSL → SPIR-V: {}".format(input_file.basename),
    )

    # Embed SPIR-V into a Rust &[u8] constant
    ctx.actions.run_shell(
        inputs = [spirv_out],
        outputs = [rust_out],
        command = """set -e
echo 'pub const SHADER: &[u8] = &' > "{out}";
xxd -i "{spirv}" | sed 's/unsigned char .* = {{/&[/; s/}};/];/' >> "{out}";
""".format(
            out = rust_out.path,
            spirv = spirv_out.path,
        ),
        progress_message = "Embedding SPIR-V → Rust array: {}".format(input_file.basename),
    )

    return [DefaultInfo(files = depset([rust_out]))]


glsl_to_rs = rule(
    implementation = _glsl_to_rs_impl,
    attrs = {
        "src": attr.label(allow_single_file = True),
        "out": attr.string(),
    },
)
