using OpenUnity.Interop;

namespace UnityEngine;

internal static class RigidbodyAccess
{
    internal enum VectorProperty { Position = 1, LinearVelocity = 2, AngularVelocity = 3 }
    internal enum ScalarProperty { Mass = 1, LinearDamping = 2, AngularDamping = 3, SleepThreshold = 4 }
    internal enum FlagProperty { UseGravity = 1, IsKinematic = 2, DetectCollisions = 3, Sleeping = 4 }

    internal static Vector3 ReadVector(Rigidbody body, VectorProperty property) => body.Context.Invoke(runtime =>
    {
        RequireCapability();
        float[] output = new float[3];
        int status = OpenUnityNative.ou_rigidbody_vector_read(runtime, body.Identity,
            (int)property, output, output.Length);
        RequireSuccess(status, "Rigidbody vector read");
        return new Vector3(output[0], output[1], output[2]);
    });

    internal static void WriteVector(Rigidbody body, VectorProperty property, Vector3 value) =>
        body.Context.Invoke(runtime =>
        {
            RequireCapability();
            float[] input = [value.x, value.y, value.z];
            int status = OpenUnityNative.ou_rigidbody_vector_write(runtime, body.Identity,
                (int)property, input, input.Length);
            RequireSuccess(status, "Rigidbody vector assignment");
            return status;
        });

    internal static float ReadScalar(Rigidbody body, ScalarProperty property) => body.Context.Invoke(runtime =>
    {
        RequireCapability();
        float output = 0f;
        int status = OpenUnityNative.ou_rigidbody_scalar_read(runtime, body.Identity, (int)property, out output);
        RequireSuccess(status, "Rigidbody scalar read");
        return output;
    });

    internal static void WriteScalar(Rigidbody body, ScalarProperty property, float value) =>
        body.Context.Invoke(runtime =>
        {
            RequireCapability();
            int status = OpenUnityNative.ou_rigidbody_scalar_write(runtime, body.Identity, (int)property, value);
            RequireSuccess(status, "Rigidbody scalar assignment");
            return status;
        });

    internal static bool ReadFlag(Rigidbody body, FlagProperty property) => body.Context.Invoke(runtime =>
    {
        RequireCapability();
        int output = 0;
        int status = OpenUnityNative.ou_rigidbody_flag_read(runtime, body.Identity, (int)property, out output);
        RequireSuccess(status, "Rigidbody flag read");
        if (output is not (0 or 1))
            throw RuntimeContext.Failure("Rigidbody flag read", output);
        return output == 1;
    });

    internal static void WriteFlag(Rigidbody body, FlagProperty property, bool value) =>
        body.Context.Invoke(runtime =>
        {
            RequireCapability();
            int status = OpenUnityNative.ou_rigidbody_flag_write(runtime, body.Identity,
                (int)property, value ? 1 : 0);
            RequireSuccess(status, "Rigidbody flag assignment");
            return status;
        });

    internal static RigidbodyInterpolation ReadInterpolation(Rigidbody body) => body.Context.Invoke(runtime =>
    {
        RequireCapability();
        int output = 0;
        int status = OpenUnityNative.ou_rigidbody_interpolation_read(runtime, body.Identity, out output);
        RequireSuccess(status, "Rigidbody interpolation read");
        if (output != 0)
            throw new NotSupportedException($"OpenUnity Rigidbody interpolation value {output} is unsupported.");
        return RigidbodyInterpolation.None;
    });

    internal static void WriteInterpolation(Rigidbody body, RigidbodyInterpolation value) =>
        body.Context.Invoke(runtime =>
        {
            RequireCapability();
            if (value != RigidbodyInterpolation.None)
                throw new NotSupportedException($"OpenUnity Rigidbody interpolation value {value} is unsupported.");
            int status = OpenUnityNative.ou_rigidbody_interpolation_write(runtime, body.Identity, 0);
            RequireSuccess(status, "Rigidbody interpolation assignment");
            return status;
        });

    internal static void WakeUp(Rigidbody body) => body.Context.Invoke(runtime =>
    {
        RequireCapability();
        int status = OpenUnityNative.ou_rigidbody_wake_up(runtime, body.Identity);
        RequireSuccess(status, "Rigidbody WakeUp");
        return status;
    });

    private static void RequireCapability() => PhysicsMotionAccess.RequireCapability();

    private static void RequireSuccess(int status, string operation)
    {
        if (status == 0)
            return;
        if (status == -2)
            throw new MissingReferenceException($"OpenUnity {operation} failed because the Rigidbody was destroyed (native status -2).");
        if (status is -8 or -18)
            throw new NotSupportedException($"OpenUnity {operation} is outside the supported Rigidbody subset (native status {status}).");
        throw RuntimeContext.Failure(operation, status);
    }
}
