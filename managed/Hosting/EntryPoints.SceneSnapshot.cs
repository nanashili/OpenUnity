using System.Buffers.Binary;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using OpenUnity.Interop;
using UnityEngine;

namespace OpenUnity.Hosting;

public static unsafe partial class EntryPoints
{
    private const int SnapshotBusy = -7;
    private const int SnapshotHeaderBytes = 16;
    private const int SnapshotRowBytes = 64;
    private const uint SnapshotPayloadVersion = 0x0001_0000;
    private const uint UnsupportedSnapshotVersion = 0;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static uint SceneSnapshotVersion()
    {
        try
        {
            uint nativeVersion = OpenUnityNative.ou_scene_snapshot_abi_version();
            return nativeVersion == OpenUnityNative.SceneSnapshotAbiVersion
                ? SnapshotPayloadVersion
                : UnsupportedSnapshotVersion;
        }
        catch
        {
            return UnsupportedSnapshotVersion;
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static int CaptureSceneSnapshot(
        long handle,
        long* ownerFileIds,
        int count,
        byte* output,
        int capacity)
    {
        if (!TryGetSnapshotPayloadSize(ownerFileIds, count, output, capacity, out int payloadBytes))
            return InvalidInput;

        try
        {
            long[] copiedOwnerIds = new ReadOnlySpan<long>(ownerFileIds, count).ToArray();
            return CaptureAfterInputCopy(handle, copiedOwnerIds, output, payloadBytes);
        }
        catch
        {
            return CallbackFailed;
        }
    }

    private static int CaptureAfterInputCopy(
        long handle,
        long[] ownerFileIds,
        byte* output,
        int payloadBytes)
    {
        bool lockTaken = false;
        try
        {
            Monitor.TryEnter(Gate, 0, ref lockTaken);
            if (!lockTaken)
                return SnapshotBusy;

            return CaptureUnderRegistryLock(handle, ownerFileIds, output, payloadBytes);
        }
        finally
        {
            if (lockTaken)
                Monitor.Exit(Gate);
        }
    }

    private static int CaptureUnderRegistryLock(
        long handle,
        long[] ownerFileIds,
        byte* output,
        int payloadBytes)
    {
        if (!Sessions.TryGetValue(handle, out Session? session))
            return InvalidHandle;
        if (activeSessionOperationDepth != 0)
            return SnapshotBusy;
        if (session.Script is not SceneController controller)
            return InvalidLifecycle;
        if (!controller.IsSnapshotOwnerThread)
            return SnapshotBusy;
        if (session.Phase != Phase.Initialized)
            return InvalidLifecycle;

        byte[] temporaryPayload = new byte[payloadBytes];
        int status = controller.CaptureSceneSnapshot(ownerFileIds, temporaryPayload);
        if (status != Success)
            return status;
        if (!IsValidSnapshotPayload(temporaryPayload, ownerFileIds))
            return CallbackFailed;

        temporaryPayload.AsSpan().CopyTo(new Span<byte>(output, payloadBytes));
        return Success;
    }

    private static bool TryGetSnapshotPayloadSize(
        long* ownerFileIds,
        int count,
        byte* output,
        int capacity,
        out int payloadBytes)
    {
        payloadBytes = 0;
        if (ownerFileIds is null || output is null || count is < 1 or > 4097 || capacity < 0)
            return false;
        if ((nuint)ownerFileIds % (nuint)sizeof(long) != 0)
            return false;

        payloadBytes = checked(SnapshotHeaderBytes + SnapshotRowBytes * count);
        if (capacity < payloadBytes)
            return false;

        nuint ownerIdBytes = (nuint)(count * sizeof(long));
        if (!TryGetAddressEnd((nuint)ownerFileIds, ownerIdBytes, out nuint ownerIdsEnd) ||
            !TryGetAddressEnd((nuint)output, (nuint)capacity, out nuint outputEnd))
            return false;

        return (nuint)ownerFileIds >= outputEnd || (nuint)output >= ownerIdsEnd;
    }

    private static bool TryGetAddressEnd(nuint start, nuint length, out nuint end)
    {
        if (length > nuint.MaxValue - start)
        {
            end = 0;
            return false;
        }

        end = start + length;
        return true;
    }

    private static bool IsValidSnapshotPayload(byte[] payload, long[] requestedIds)
    {
        ReadOnlySpan<byte> bytes = payload;
        int expectedLength = SnapshotHeaderBytes + SnapshotRowBytes * requestedIds.Length;
        if (bytes.Length != expectedLength ||
            BinaryPrimitives.ReadUInt32LittleEndian(bytes) != SnapshotPayloadVersion ||
            BinaryPrimitives.ReadInt32LittleEndian(bytes[4..]) < 0 ||
            BinaryPrimitives.ReadInt32LittleEndian(bytes[8..]) != requestedIds.Length ||
            BinaryPrimitives.ReadUInt32LittleEndian(bytes[12..]) != SnapshotRowBytes)
            return false;

        for (int rowIndex = 0; rowIndex < requestedIds.Length; rowIndex++)
        {
            if (!IsValidSnapshotRow(bytes, rowIndex, requestedIds[rowIndex]))
                return false;
        }

        return true;
    }

    private static bool IsValidSnapshotRow(ReadOnlySpan<byte> payload, int rowIndex, long requestedId)
    {
        int rowOffset = SnapshotHeaderBytes + SnapshotRowBytes * rowIndex;
        ReadOnlySpan<byte> row = payload.Slice(rowOffset, SnapshotRowBytes);
        if (BinaryPrimitives.ReadInt64LittleEndian(row) != requestedId)
            return false;

        int state = BinaryPrimitives.ReadInt32LittleEndian(row[8..]);
        int unitScaleChain = BinaryPrimitives.ReadInt32LittleEndian(row[12..]);
        if (state is < 0 or > 2 || unitScaleChain is < 0 or > 1)
            return false;

        bool isDead = state == 0;
        if (isDead && unitScaleChain != 0)
            return false;

        for (int coefficientIndex = 0; coefficientIndex < 12; coefficientIndex++)
        {
            int bits = BinaryPrimitives.ReadInt32LittleEndian(row[(16 + coefficientIndex * sizeof(float))..]);
            if (isDead ? bits != 0 : !float.IsFinite(BitConverter.Int32BitsToSingle(bits)))
                return false;
        }

        return true;
    }
}
