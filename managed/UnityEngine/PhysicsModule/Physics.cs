using OpenUnity.Interop;

namespace UnityEngine;

public class Physics
{
    private static readonly Lazy<bool> GravityCapability = new(() =>
    {
        uint abi = OpenUnityNative.ou_physics_gravity_abi_version();
        if (abi != OpenUnityNative.PhysicsGravityAbiVersion)
            throw new NotSupportedException(
                $"OpenUnity Physics gravity ABI 1.0 is required; received 0x{abi:X8}.");
        return true;
    });

    public Physics()
    {
        throw new NotSupportedException("OpenUnity Physics construction is unsupported.");
    }

    public static Vector3 gravity
    {
        get
        {
            RuntimeContext context = RuntimeContext.Shared;
            return context.Invoke(runtime =>
            {
                _ = GravityCapability.Value;

                float[] output = new float[3];
                int status = OpenUnityNative.ou_physics_gravity_read(runtime, output, output.Length);
                if (status != 0)
                    throw RuntimeContext.Failure("Physics gravity read", status);

                return new Vector3(output[0], output[1], output[2]);
            });
        }
        set
        {
            RuntimeContext context = RuntimeContext.Shared;
            context.Invoke(runtime =>
            {
                _ = GravityCapability.Value;

                float[] input = [value.x, value.y, value.z];
                int status = OpenUnityNative.ou_physics_gravity_write(runtime, input, input.Length);
                if (status != 0)
                    throw RuntimeContext.Failure("Physics gravity assignment", status);

                return status;
            });
        }
    }
}
