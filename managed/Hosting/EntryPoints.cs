using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Reflection;
using System.Text;
using System.Runtime.Loader;
using System.Text.Json;

namespace OpenUnity.Hosting;

public static unsafe class EntryPoints
{
    public const uint AbiVersion = 0x00010000;
    public const int Success = 0;
    public const int InvalidHandle = -1;
    public const int InvalidInput = -2;
    public const int Exhausted = -4;
    public const int InvalidLifecycle = -5;
    public const int CallbackFailed = -6;

    private const int InitializeOpcode = 1;
    private const int TickOpcode = 2;
    private const int ShutdownOpcode = 3;
    private static readonly object Gate = new();
    private static readonly Dictionary<long, Session> Sessions = new();
    private static long _lastHandle;
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static uint Version() => AbiVersion;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static uint GameplayVersion() => 0x00010000;

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static uint SceneVersion() => 0x00010000;

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
    public static uint TimedSceneVersion() => 0x00010000;

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

    private static Assembly LoadUserAssembly(string path)
    {
        string fullPath = Path.GetFullPath(path);
        AssemblyLoadContext context = AssemblyLoadContext.GetLoadContext(typeof(EntryPoints).Assembly)
            ?? AssemblyLoadContext.Default;
        return context.Assemblies.FirstOrDefault(candidate =>
            !string.IsNullOrEmpty(candidate.Location) &&
            string.Equals(Path.GetFullPath(candidate.Location), fullPath, StringComparison.Ordinal))
            ?? context.LoadFromAssemblyPath(fullPath);
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static long Create(byte* assemblyPathUtf8, byte* typeNameUtf8) =>
        CreateSession(assemblyPathUtf8, typeNameUtf8, gameplay: false);

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static long CreateGameplay(byte* assemblyPathUtf8, byte* typeNameUtf8) =>
        CreateSession(assemblyPathUtf8, typeNameUtf8, gameplay: true);

    private static long CreateSession(byte* assemblyPathUtf8, byte* typeNameUtf8, bool gameplay)
    {
        try
        {
            if (assemblyPathUtf8 is null || typeNameUtf8 is null)
                return InvalidInput;

            string? path = ReadUtf8(assemblyPathUtf8);
            string? typeName = ReadUtf8(typeNameUtf8);
            if (string.IsNullOrWhiteSpace(path) || string.IsNullOrWhiteSpace(typeName) || !Path.IsPathFullyQualified(path) || !File.Exists(path))
                return InvalidInput;

            Assembly assembly = LoadUserAssembly(path);
            Type? scriptType = assembly.GetType(typeName, throwOnError: false, ignoreCase: false);
            if (scriptType is null || !scriptType.IsClass || scriptType.IsAbstract ||
                scriptType.ContainsGenericParameters || scriptType.GetConstructor(Type.EmptyTypes) is null ||
                (gameplay ? scriptType == typeof(UnityEngine.MonoBehaviour) ||
                    !typeof(UnityEngine.MonoBehaviour).IsAssignableFrom(scriptType) :
                    !typeof(IHostedScript).IsAssignableFrom(scriptType)))
                return InvalidInput;

            IHostedScript? script = gameplay ? new GameplayController(scriptType) :
                Activator.CreateInstance(scriptType) as IHostedScript;
            if (script is null)
                return InvalidInput;

            lock (Gate)
            {
                if (_lastHandle == long.MaxValue)
                    return Exhausted;
                long handle = ++_lastHandle;
                Sessions.Add(handle, new Session(script));
                return handle;
            }
        }
        catch (ArgumentException)
        {
            return InvalidInput;
        }
        catch (FileNotFoundException)
        {
            return InvalidInput;
        }
        catch (BadImageFormatException)
        {
            return InvalidInput;
        }
        catch (TypeLoadException)
        {
            return InvalidInput;
        }
        catch
        {
            return CallbackFailed;
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static int Invoke(long handle, int opcode, long value, long* output)
    {
        try
        {
            if (output is null)
                return InvalidInput;

            lock (Gate)
            {
                if (!Sessions.TryGetValue(handle, out Session? session))
                    return InvalidHandle;
                if (opcode is not (InitializeOpcode or TickOpcode or ShutdownOpcode))
                    return InvalidInput;
                if (session.Phase is Phase.Faulted or Phase.Destroyed)
                    return InvalidLifecycle;

                try
                {
                    long result;
                    switch (opcode)
                    {
                        case InitializeOpcode when session.Phase == Phase.Created:
                            result = session.Script.Initialize(value);
                            session.Phase = Phase.Initialized;
                            break;
                        case TickOpcode when session.Phase == Phase.Initialized:
                            result = session.Script.Tick(value);
                            break;
                        case ShutdownOpcode when session.Phase == Phase.Initialized:
                            result = session.Script.Shutdown(value);
                            session.Phase = Phase.Shutdown;
                            break;
                        case InitializeOpcode or TickOpcode or ShutdownOpcode:
                            return InvalidLifecycle;
                        default:
                            return InvalidInput;
                    }

                    *output = result;
                    return Success;
                }
                catch
                {
                    session.Phase = Phase.Faulted;
                    return CallbackFailed;
                }
            }
        }
        catch
        {
            return CallbackFailed;
        }
    }

    [UnmanagedCallersOnly(CallConvs = new[] { typeof(CallConvCdecl) })]
    public static int Destroy(long handle)
    {
        try
        {
            lock (Gate)
            {
                if (!Sessions.TryGetValue(handle, out Session? session))
                    return InvalidHandle;
                if (session.Script is SceneController controller)
                    controller.Dispose();
                Sessions.Remove(handle);
                session.Phase = Phase.Destroyed;
                return Success;
            }
        }
        catch
        {
            return CallbackFailed;
        }
    }

    private static string? ReadUtf8(byte* pointer)
    {
        int length = 0;
        while (pointer[length] != 0)
            length++;
        return StrictUtf8.GetString(pointer, length);
    }

    private enum Phase
    {
        Created,
        Initialized,
        Shutdown,
        Faulted,
        Destroyed,
    }

    private sealed class Session(IHostedScript script)
    {
        public IHostedScript Script { get; } = script;
        public Phase Phase { get; set; } = Phase.Created;
    }
}
