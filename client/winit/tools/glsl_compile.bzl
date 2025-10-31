def _glsl_to_spirv_impl(ctx):
    input_file = ctx.file.src
    output_file =  ctx.actions.declare_file(input_file.basename + ".spv")

    ctx.actions.run_shell(
        inputs = [input_file],
        outputs = [output_file],
        command = "glslangValidator -V -o {spirv_out} {input_file}".format(
            spirv_out = output_file.path,
            input_file = input_file.path,
        ),
    )

    return [DefaultInfo(files = depset([output_file]))]


glsl_to_spirv = rule(
    implementation = _glsl_to_spirv_impl,
    attrs = {
        "src": attr.label(allow_single_file = True),
    },
)

def _embed_rs_impl(ctx):
    input_file = ctx.file.src
    output_file =  ctx.actions.declare_file(ctx.label.name + ".rs")

    ctx.actions.run(
        inputs = [input_file],
        outputs = [output_file],
        arguments = [
            input_file.path,
            output_file.path,
            ctx.attr.var_name,
        ],
        executable = ctx.executable._tool,  
        progress_message = "Embedding blob into Rust: {}".format(input_file.basename),
    )

    return [DefaultInfo(files = depset([output_file]))]


embed_rs = rule(
    implementation = _embed_rs_impl,
    attrs = {
        "src": attr.label(allow_single_file = True),
        "var_name": attr.string(),
         "_tool": attr.label(
            default = Label("//client/winit/tools:embed_rs"),
            cfg = "exec",
            executable = True,
        ),
    },
)