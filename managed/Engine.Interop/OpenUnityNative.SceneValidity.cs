using System.Runtime.InteropServices;

namespace OpenUnity.Interop;

internal static partial class OpenUnityNative
{
    internal const uint SceneValidityAbiVersion = 0x0001_0000;

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_scene_validity_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern int ou_scene_is_valid(long runtime, long scene);
}
