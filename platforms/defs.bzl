"""The execution platform of the pgtk build.

Every action runs locally. When the remote cache is on, the platform also reads from and uploads
to the BuildBuddy cache that tools/buck2 configures.

The remote execution properties are part of the key of every action, also for a local action
with a remote cache. They hold what Buck does not hash itself: the OS, the CPU architecture and
the toolchain ID, a hash of the system toolchain that tools/buck2 calculates. So the cache never
gives an output to another OS, CPU or toolchain.
"""

load("@prelude//cfg/exec_platform:marker.bzl", "get_exec_platform_marker")

def _execution_platform_impl(ctx: AnalysisContext) -> list[Provider]:
    if ctx.attrs.remote_cache and not ctx.attrs.toolchain_id:
        fail("the remote cache needs a toolchain ID; run buck2 through tools/buck2")

    constraints = dict()
    constraints.update(ctx.attrs.cpu_configuration[ConfigurationInfo].constraints)
    constraints.update(ctx.attrs.os_configuration[ConfigurationInfo].constraints)
    cfg = ConfigurationInfo(constraints = constraints, values = {})

    os = host_info().os
    name = ctx.label.raw_target()
    platform = ExecutionPlatformInfo(
        label = name,
        configuration = cfg,
        executor_config = CommandExecutorConfig(
            local_enabled = True,
            remote_enabled = False,
            remote_cache_enabled = ctx.attrs.remote_cache,
            allow_cache_uploads = ctx.attrs.remote_cache,
            remote_execution_properties = {
                "OSFamily": "Darwin" if os.is_macos else "Linux",
                "pgtk-arch": "arm64" if host_info().arch.is_aarch64 else "x86_64",
                "pgtk-toolchain": ctx.attrs.toolchain_id,
            },
            remote_execution_use_case = "buck2-default",
        ),
    )

    return [
        DefaultInfo(),
        platform,
        PlatformInfo(label = str(name), configuration = cfg),
        ExecutionPlatformRegistrationInfo(
            platforms = [platform],
            exec_marker_constraint = get_exec_platform_marker(),
        ),
    ]

execution_platform = rule(
    impl = _execution_platform_impl,
    attrs = {
        "cpu_configuration": attrs.dep(providers = [ConfigurationInfo]),
        "os_configuration": attrs.dep(providers = [ConfigurationInfo]),
        "remote_cache": attrs.bool(),
        "toolchain_id": attrs.string(),
    },
)
