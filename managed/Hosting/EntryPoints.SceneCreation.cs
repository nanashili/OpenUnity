using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.Json;

namespace OpenUnity.Hosting;

public static unsafe partial class EntryPoints
{
    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static long CreateScene(byte* assemblyPathUtf8, byte* planUtf8, int byteCount)
    {
        try
        {
            UnityEngine.RuntimeContext.RequireEngineAccess();
            if (assemblyPathUtf8 is null || planUtf8 is null || byteCount is <= 0 or > ScenePlan.MaximumBytes)
                return InvalidInput;
            string? path = ReadUtf8(assemblyPathUtf8);
            if (string.IsNullOrWhiteSpace(path) || !Path.IsPathFullyQualified(path) || !File.Exists(path))
                return InvalidInput;
            byte[] payload = new ReadOnlySpan<byte>(planUtf8, byteCount).ToArray();
            SceneController controller = new(LoadUserAssembly(path), payload);
            lock (Gate)
            {
                if (_lastHandle == long.MaxValue)
                    return Exhausted;
                long handle = ++_lastHandle;
                Sessions.Add(handle, new Session(controller));
                return handle;
            }
        }
        catch (Exception error) when (error is ArgumentException or JsonException or FileNotFoundException or
            BadImageFormatException or TypeLoadException or NotSupportedException)
        {
            return InvalidInput;
        }
        catch
        {
            return CallbackFailed;
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static long CreateTimedScene(byte* assemblyPathUtf8, byte* planUtf8, int byteCount,
        int source, double step)
    {
        try
        {
            UnityEngine.RuntimeContext.RequireEngineAccess();
            if (assemblyPathUtf8 is null || planUtf8 is null || byteCount is <= 0 or > ScenePlan.MaximumBytes)
                return InvalidInput;
            if (!double.IsFinite(step) ||
                (source == 0 ? step != 0 : source != 1 || step < 0.000001 || step > 60))
                return InvalidInput;
            string? path = ReadUtf8(assemblyPathUtf8);
            if (string.IsNullOrWhiteSpace(path) || !Path.IsPathFullyQualified(path) || !File.Exists(path))
                return InvalidInput;
            byte[] payload = new ReadOnlySpan<byte>(planUtf8, byteCount).ToArray();
            lock (Gate)
            {
                if (_lastHandle == long.MaxValue)
                    return Exhausted;
                SceneController controller = new(LoadUserAssembly(path), payload, source, step);
                long handle = ++_lastHandle;
                Sessions.Add(handle, new Session(controller));
                return handle;
            }
        }
        catch (Exception error) when (error is ArgumentException or JsonException or FileNotFoundException or
            BadImageFormatException or TypeLoadException or NotSupportedException)
        {
            return InvalidInput;
        }
        catch
        {
            return CallbackFailed;
        }
    }
}
