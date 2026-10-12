using System.Runtime.InteropServices;

namespace OpenUnity.Interop;

internal static partial class OpenUnityNative
{
    internal const uint PhysicsMotionAbiVersion = 0x0001_0000;

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_physics_motion_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_rigidbody_reserve(long runtime, long owner);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_commit(long runtime, long body);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_abort(long runtime, long body);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_physics_component_count(long runtime, long owner);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_physics_component_at(long runtime, long owner, int index);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_vector_read(
        long runtime,
        long body,
        int property,
        [Out, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 4)] float[] output,
        int count);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_vector_write(
        long runtime,
        long body,
        int property,
        [In, MarshalAs(UnmanagedType.LPArray, SizeParamIndex = 4)] float[] input,
        int count);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_scalar_read(
        long runtime,
        long body,
        int property,
        out float output);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_scalar_write(
        long runtime,
        long body,
        int property,
        float value);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_flag_read(
        long runtime,
        long body,
        int property,
        out int output);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_flag_write(
        long runtime,
        long body,
        int property,
        int value);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_interpolation_read(
        long runtime,
        long body,
        out int output);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_interpolation_write(
        long runtime,
        long body,
        int value);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_rigidbody_wake_up(long runtime, long body);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_physics_mode_read(long runtime);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_physics_mode_write(long runtime, int mode);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_physics_sync_transforms(long runtime);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_physics_simulate(long runtime, float duration);
}
