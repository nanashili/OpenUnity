using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace OpenUnity.Hosting;

public static unsafe partial class EntryPoints
{
    public const uint AbiVersion = 0x00010000;
    public const int Success = 0;
    public const int InvalidHandle = -1;
    public const int InvalidInput = -2;
    public const int Exhausted = -4;
    public const int InvalidLifecycle = -5;
    public const int CallbackFailed = -6;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static uint Version() => AbiVersion;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static uint GameplayVersion() => 0x00010000;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static uint SceneVersion() => 0x00010000;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static uint TimedSceneVersion() => 0x00010000;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static long Create(byte* assemblyPathUtf8, byte* typeNameUtf8) =>
        CreateSession(assemblyPathUtf8, typeNameUtf8, gameplay: false);

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static long CreateGameplay(byte* assemblyPathUtf8, byte* typeNameUtf8) =>
        CreateSession(assemblyPathUtf8, typeNameUtf8, gameplay: true);
}
