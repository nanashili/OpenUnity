using System.Runtime.InteropServices;

namespace OpenUnity.Interop;

internal static partial class OpenUnityNative
{
    internal const uint SceneSnapshotAbiVersion = 0x0001_0000;

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern uint ou_scene_snapshot_abi_version();

    [DllImport("openunity_engine", CallingConvention = CallingConvention.Cdecl, ExactSpelling = true)]
    internal static extern unsafe int ou_scene_snapshot(
        long runtime,
        long scene,
        long* ownerFileIds,
        int count,
        byte* output,
        int capacity);
}
