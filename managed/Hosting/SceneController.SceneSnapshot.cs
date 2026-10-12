using OpenUnity.Interop;
using UnityEngine;

namespace OpenUnity.Hosting;

internal sealed partial class SceneController
{
    internal bool IsSnapshotOwnerThread => dispatcher.IsOwnerThread;

    internal int CaptureSceneSnapshot(long[] ownerFileIds, byte[] output)
    {
        dispatcher.CheckThread();

        if (!published || scene <= 0)
            return EntryPoints.InvalidLifecycle;

        uint version = OpenUnityNative.ou_scene_snapshot_abi_version();
        if (version != OpenUnityNative.SceneSnapshotAbiVersion)
            return EntryPoints.CallbackFailed;

        int nativeStatus = context.Invoke(runtime => CaptureNativeSnapshot(
            runtime,
            scene,
            ownerFileIds,
            output));

        return MapNativeStatus(nativeStatus);
    }

    private static unsafe int CaptureNativeSnapshot(
        long runtime,
        long scene,
        long[] ownerFileIds,
        byte[] output)
    {
        fixed (long* ownerFileIdsPointer = ownerFileIds)
        fixed (byte* outputPointer = output)
        {
            return OpenUnityNative.ou_scene_snapshot(
                runtime,
                scene,
                ownerFileIdsPointer,
                ownerFileIds.Length,
                outputPointer,
                output.Length);
        }
    }

    private static int MapNativeStatus(int status) => status switch
    {
        0 => EntryPoints.Success,
        -1 => -101,
        -2 => -102,
        -3 => -103,
        -5 => -105,
        -7 => -107,
        -8 => -108,
        -9 => -109,
        -12 => -112,
        -13 => -113,
        -14 => -114,
        -15 => -115,
        -17 => -117,
        _ => EntryPoints.CallbackFailed,
    };
}
