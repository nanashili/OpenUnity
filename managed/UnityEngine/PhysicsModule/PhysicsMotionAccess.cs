using OpenUnity.Interop;

namespace UnityEngine;

internal static class PhysicsMotionAccess
{
    internal const uint MotionAbiVersion = 0x0001_0000;

    internal static void RequireCapability()
    {
        uint version = OpenUnityNative.ou_physics_motion_abi_version();
        if (version != MotionAbiVersion)
            throw new NotSupportedException(
                $"OpenUnity Physics motion ABI 1.0 is required; received 0x{version:X8}.");
    }

    internal static SimulationMode ReadMode(long runtime)
    {
        int status = OpenUnityNative.ou_physics_mode_read(runtime);
        return status switch
        {
            0 => SimulationMode.FixedUpdate,
            1 => SimulationMode.Script,
            _ => throw StatusFailure("Physics simulation mode read", status)
        };
    }

    internal static void WriteMode(long runtime, int mode)
    {
        int status = OpenUnityNative.ou_physics_mode_write(runtime, mode);
        RequireSuccess("Physics simulation mode assignment", status);
    }

    internal static void SyncTransforms(long runtime)
    {
        int status = OpenUnityNative.ou_physics_sync_transforms(runtime);
        RequireSuccess("Physics transform synchronization", status);
    }

    internal static void Simulate(long runtime, float step)
    {
        int status = OpenUnityNative.ou_physics_simulate(runtime, step);
        RequireSuccess("Physics simulation", status);
    }

    private static void RequireSuccess(string operation, int status)
    {
        if (status != 0)
            throw StatusFailure(operation, status);
    }

    private static Exception StatusFailure(string operation, int status) => status switch
    {
        -2 => new MissingReferenceException($"OpenUnity {operation} failed (native status {status})."),
        -8 or -18 => new NotSupportedException($"OpenUnity {operation} is unsupported (native status {status})."),
        _ when status < 0 => RuntimeContext.Failure(operation, status),
        _ => new InvalidOperationException($"OpenUnity {operation} returned unexpected native status {status}.")
    };
}
