using OpenUnity.Interop;

namespace UnityEngine;

internal sealed partial class RuntimeContext
{
    private static readonly Lazy<bool> SceneValidityCapability = new(() =>
    {
        try
        {
            uint abi = OpenUnityNative.ou_scene_validity_abi_version();
            if (abi != OpenUnityNative.SceneValidityAbiVersion)
                throw new NotSupportedException(
                    $"OpenUnity Scene Validity ABI 1.0 is required; received 0x{abi:X8}.");
            return true;
        }
        catch (EntryPointNotFoundException error)
        {
            throw new NotSupportedException("OpenUnity Scene Validity ABI 1.0 is unavailable.", error);
        }
    });

    internal void RequireSceneValidity() => _ = SceneValidityCapability.Value;
}
