using System.Runtime.InteropServices;

namespace OpenUnity.Interop;

[StructLayout(LayoutKind.Sequential, Pack = 8, Size = 16)]
internal struct SceneRootIdentity
{
    internal long GameObject;
    internal long Transform;
}

internal static partial class OpenUnityNative
{
    internal const uint SceneRootsAbiVersion = 0x0001_0000;

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_scene_roots_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern long ou_gameobject_scene(long runtime, long gameObject);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_scene_root_count(long runtime, long scene);

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern unsafe int ou_scene_roots_read(
        long runtime,
        long scene,
        SceneRootIdentity* output,
        int capacity);
}
