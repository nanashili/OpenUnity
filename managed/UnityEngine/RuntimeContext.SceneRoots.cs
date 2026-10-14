using OpenUnity.Interop;

namespace UnityEngine;

internal sealed partial class RuntimeContext
{
    private static readonly Lazy<bool> SceneRootsCapability = new(() =>
    {
        try
        {
            uint abi = OpenUnityNative.ou_scene_roots_abi_version();
            if (abi != OpenUnityNative.SceneRootsAbiVersion)
                throw new NotSupportedException(
                    $"OpenUnity Scene Roots ABI 1.0 is required; received 0x{abi:X8}.");
            return true;
        }
        catch (EntryPointNotFoundException error)
        {
            throw new NotSupportedException("OpenUnity Scene Roots ABI 1.0 is unavailable.", error);
        }
    });

    internal void RequireSceneRoots() => _ = SceneRootsCapability.Value;
}
